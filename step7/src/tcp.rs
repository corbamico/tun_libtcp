#![allow(dead_code)]
use super::DataReceiver;
use bytes::{Bytes, BytesMut};
use etherparse::*;
use futures::future::Future;
use futures::sink::Sink;
use futures::sync::mpsc;

use std::net::{Ipv4Addr, SocketAddrV4};

#[derive(Debug, PartialEq)]
enum State {
    Listening,
    SynRcvd,
    Estab,
    CloseWait,
    LastAck,
    Closed,
}

pub enum FilterPacket {
    ///ICMP Ping Request Packet
    Ping(BytesMut),
    ///TCP SYN Packet
    TcpSyn(Ipv4Header, TcpHeader),
    ///TCP Packet
    Tcp(SocketAddrV4, TcpHeader, Bytes),
}

pub struct Connection {
    state: State,
    send: SendSequenceSpace,
    recv: RecvSequenceSpace,
    pub src: SocketAddrV4,
    pub dst: SocketAddrV4,
    ///rx_tun used in libtcp::process.
    tx_tun: mpsc::Sender<Bytes>,
    //tx_data, from libtcp sending data to application.
    tx_data: Option<mpsc::Sender<Bytes>>,
}

impl Connection {
    pub fn new(tx_tun: mpsc::Sender<Bytes>) -> Connection {
        Connection {
            state: State::Listening,
            send: SendSequenceSpace::default(),
            recv: RecvSequenceSpace::default(),
            src: SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 0),
            dst: SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 0),
            tx_tun,
            tx_data: None,
        }
    }

    ///on_packet
    /// 1.change state on Connect
    /// 2.send back syn/ack or fin automaticlly
    /// 3.send payload to application-level
    pub fn on_packet(
        &mut self,
        tcph: &TcpHeader,
        payload: &[u8],
        tx_accept: mpsc::Sender<DataReceiver>,
    ) {
        //eprintln!("on_packet, {:?}", self.state);
        match self.state {
            State::SynRcvd => {
                //we can not handle disordered packet.
                if tcph.fin || tcph.psh || !tcph.ack || !payload.is_empty() {
                    return;
                }

                //we finish 3-handshake.
                let (tx_data, rx_data) = mpsc::channel(2);
                tx_accept.send(rx_data).wait().unwrap();
                self.tx_data = Some(tx_data);
                self.state = State::Estab;
            }
            State::Estab => {
                //send data to application-level, if payload not empty
                let payload_len = payload.len() as u32;
                if !payload.is_empty() {
                    let tx = self.tx_data.as_ref().unwrap().clone();
                    tx.send(Bytes::from(payload)).wait().unwrap();
                    //TODO:
                    //should close/drop, if application TcpStream closed.
                }
                //then ,we need send back ACK packet.
                self.send_ack(false, tcph.fin, payload_len);
            }
            State::LastAck => {
                if tcph.acknowledgment_number == self.send.nxt {
                    self.state = State::Closed;
                }
            }
            _ => {}
        }
    }
    pub fn on_packet_syn(&mut self, iph: &Ipv4Header, tcph: &TcpHeader) {
        match self.state {
            State::Listening => {
                if tcph.fin || tcph.psh || tcph.ack || !tcph.syn {
                    return;
                }
                self.init_state(iph, tcph);
                self.send_ack(true, false, 0);
            }
            _ => {}
        }
    }

    fn init_state(&mut self, iph: &Ipv4Header, tcph: &TcpHeader) {
        //only accept syn packet, if tcph.syn

        let iss = 0;
        let wnd = 1024;
        let ack = tcph.sequence_number + 1;

        self.state = State::SynRcvd;
        self.src = SocketAddrV4::new(Ipv4Addr::from(iph.destination), tcph.destination_port);
        self.dst = SocketAddrV4::new(Ipv4Addr::from(iph.source), tcph.source_port);
        self.send = SendSequenceSpace {
            iss,
            una: iss,
            nxt: tcph.acknowledgment_number,
            wnd: wnd,
            up: false,
            wl1: 0,
            wl2: 0,
        };
        self.recv = RecvSequenceSpace {
            irs: tcph.sequence_number,
            nxt: ack,
            wnd: tcph.window_size,
            up: false,
        };
    }

    fn send_ack(&mut self, syn_recieved: bool, fin_recieved: bool, payload_len: u32) {
        if fin_recieved {
            // [FIN] should count seq + 1
            self.recv.nxt = self.recv.nxt + 1;
        } else {
            self.recv.nxt = self.recv.nxt + payload_len;
        }

        let payload = &[0u8; 0];
        let mut builder = PacketBuilder::ipv4(self.src.ip().octets(), self.dst.ip().octets(), 20)
            .tcp(
                self.src.port(),
                self.dst.port(),
                self.send.nxt,
                self.send.wnd,
            )
            .ack(self.recv.nxt);

        if syn_recieved {
            builder = builder.syn();
        }

        if fin_recieved {
            builder = builder.fin();
        }

        let mut result = Vec::<u8>::with_capacity(payload.len());
        builder.write(&mut result, payload).unwrap();

        let sent_bytes = Bytes::from(result);
        self.tx_tun.try_send(sent_bytes).unwrap();

        if syn_recieved {
            self.send.nxt = self.send.nxt + 1;
        }

        if fin_recieved {
            self.state = State::LastAck;
            self.send.nxt = self.send.nxt + 1;
        }
    }

    fn gen_packet(&mut self, tcph: &TcpHeader, payload: &[u8]) -> Bytes {
        let mut builder = PacketBuilder::ipv4(self.src.ip().octets(), self.dst.ip().octets(), 20)
            .tcp(
                self.src.port(),
                self.dst.port(),
                self.send.nxt,
                self.send.wnd,
            )
            .ack(self.recv.nxt);

        if tcph.syn {
            builder = builder.syn();
        }

        if tcph.fin {
            builder = builder.fin();
        }

        let mut result = Vec::<u8>::with_capacity(payload.len());
        builder.write(&mut result, payload).unwrap();

        Bytes::from(result)
    }

    pub fn is_closed(&self) -> bool {
        self.state == State::Closed
    }
}

#[derive(Debug, Default)]
struct SendSequenceSpace {
    una: u32,
    nxt: u32,
    wnd: u16,
    up: bool,
    wl1: usize,
    wl2: usize,
    iss: u32,
}
#[derive(Debug, Default)]
struct RecvSequenceSpace {
    nxt: u32,
    wnd: u16,
    up: bool,
    irs: u32,
}
