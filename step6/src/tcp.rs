
#![allow(dead_code)]
use bytes::{Bytes,BytesMut};
use std::net::{SocketAddrV4,Ipv4Addr};
use etherparse::*;
enum State{
    SynRcvd,
    Estab,
}

pub enum FilterPacket{
    Ping(BytesMut),
    Tcp(Connection),
}


pub struct Connection {
    state: State,
    send: SendSequenceSpace,
    recv: RecvSequenceSpace,
    src:  std::net::SocketAddrV4,
    dst:  std::net::SocketAddrV4,  
    pub tcph: TcpHeader,     
}

impl Connection {
    pub fn new(iph: &Ipv4Header,tcph: &TcpHeader)->Connection{
        let iss = 0;
        let wnd = 1024;

        let ack = if !tcph.psh {tcph.sequence_number+1} else {tcph.sequence_number +  iph.payload_len as u32 - tcph.header_len() as u32 };
        //only handle first ack
        let state = if tcph.ack {State::Estab} else {State::SynRcvd};

        let con = Connection{
            state: state,
            send: SendSequenceSpace{
                iss,
                una:iss,
                nxt:tcph.acknowledgment_number,
                wnd:wnd,
                up:false,
                wl1:0,
                wl2:0,
            },
            recv: RecvSequenceSpace{
                irs:tcph.sequence_number,
                nxt:ack,
                wnd:tcph.window_size,
                up:false,
            },
            src: SocketAddrV4::new(Ipv4Addr::from(iph.destination), tcph.destination_port),
            dst: SocketAddrV4::new(Ipv4Addr::from(iph.source), tcph.source_port),
            tcph: tcph.clone(),
        };        
        con
    }
    pub fn gen_packet(&mut self,payload :&[u8])->Bytes{
        let mut builder = PacketBuilder::ipv4(self.src.ip().octets(),self.dst.ip().octets(),20)
                                    .tcp(self.src.port()
                                        ,self.dst.port()
                                        ,self.send.nxt
                                        ,self.send.wnd)
                                    .ack(self.recv.nxt)
                                    ;                                    
                                   
        if self.tcph.syn {            
            builder = builder.syn();
        }


        if self.tcph.fin {
            builder = builder.fin();    
        }

        let mut result = Vec::<u8>::with_capacity(payload.len());
        builder.write(&mut result,payload).unwrap();
        
        Bytes::from(result)        
    }
}


struct SendSequenceSpace{
    una:u32,
    nxt:u32,
    wnd:u16,
    up:bool,
    wl1:usize,
    wl2:usize,
    iss:u32,
}

struct RecvSequenceSpace{
    nxt:u32,
    wnd:u16,
    up:bool,
    irs:u32,
}