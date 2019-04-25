#![allow(dead_code)]
use futures::sink::Sink;
use futures::stream::Stream;
use futures::{Async, Future, Poll};

use std::collections::HashMap;
use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;

//use std::sync::mpsc;
use futures::sync::mpsc;

use tun::platform::Device;

use tokio::codec::{BytesCodec, FramedRead, FramedWrite};
use tokio::io::AsyncRead;
use tokio::reactor::PollEvented2;
use tokio::runtime::Runtime;

use bytes::{Bytes, BytesMut};

use etherparse::*;

mod tcp;

type DataReceiver = mpsc::Receiver<Bytes>;

#[derive(Debug)]
pub struct TcpListener {
    //channel for notify incoming socket.
    rx_accept: mpsc::Receiver<DataReceiver>,
}

#[derive(Debug)]
pub struct Incoming {
    rx_accept: mpsc::Receiver<DataReceiver>,
}

#[derive(Debug)]
pub struct TcpStream {
    rx_data: mpsc::Receiver<Bytes>,
}

impl TcpListener {
    pub fn bind(libtcp: &mut Libtcp, _addr: &SocketAddr) -> io::Result<Self> {
        let (tx, rx) = mpsc::channel(1500);

        //put sender of channel into libtcp
        libtcp.insert(tx);

        Ok(TcpListener { rx_accept: rx })
        //unimplemented!()
    }
    pub fn incoming(self) -> Incoming {
        Incoming {
            rx_accept: self.rx_accept,
        }
        //unimplemented!()
    }
}

impl Stream for Incoming {
    type Item = TcpStream;
    //type Error = io::Error;
    type Error = ();
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.rx_accept.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::Ready(Some(rx_data))) => Ok(Async::Ready(Some(TcpStream { rx_data }))),
            Err(_) => Err(()),
        }
    }
}

impl Stream for TcpStream {
    type Item = Bytes;
    type Error = ();
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.rx_data.poll()
    }
}

///====== tcp protocol engine ======
pub struct Libtcp {
    tx_accept: Option<Arc<mpsc::Sender<DataReceiver>>>,
    conns: HashMap<SocketAddrV4, tcp::Connection>,
}

impl Libtcp {
    pub fn instance() -> Self {
        Libtcp {
            tx_accept: None,
            conns: HashMap::new(),
        }
    }

    pub(crate) fn insert(&mut self, tx: mpsc::Sender<DataReceiver>) {
        self.tx_accept.replace(Arc::new(tx));
    }

    // pub fn process_in_runtime(&mut self, rt: &mut Runtime) -> io::Result<()> {
    //     let task = self.process();
    //     rt.spawn(task);
    //     Ok(())
    // }
    pub fn process(mut self) -> impl Future<Item = (), Error = ()> {
        let event = PollEvented2::new(Device::default());
        let (rd, wr) = event.split();
        let writer = FramedWrite::new(wr, BytesCodec::new());

        let (tx, rx) = mpsc::channel(1500);
        let tx_accept = self.tx_accept.as_ref().unwrap().clone();

        let reader_fut = FramedRead::new(rd, BytesCodec::new())
            .filter_map(|bytes| {
                let packet = Self::filter_packet(bytes);
                packet
            })
            //Step 2: handle icmp packet
            .map(move |packet| {
                match packet {
                    tcp::FilterPacket::Ping(bytes) => {
                        let echo_bytes = Self::gen_ping_echo(bytes);
                        tx.clone().send(echo_bytes).wait().unwrap();
                    }
                    tcp::FilterPacket::Tcp(_src, _tcph, _bytes) => {
                        //step1: get connection from Hashmap
                        if let Some(conn) = self.conns.get_mut(&_src) {
                            let _s = (*tx_accept).clone();
                            conn.on_packet(&_tcph, &_bytes, _s);
                            //connection closed, remove it from hashmap.
                            if conn.is_closed(){
                                //eprintln!("connection deleted");
                                self.conns.remove(&_src);
                            }
                        }
                    }
                    tcp::FilterPacket::TcpSyn(_iph, _tcph) => {
                        //step 1:create new connection
                        let mut conn = tcp::Connection::new(tx.clone());
                        conn.on_packet_syn(&_iph, &_tcph);
                        //step 2:put connection into hashmap
                        self.conns.insert(conn.dst, conn);
                    }
                }

                ()
            })
            .collect();

        let rx_adapter = rx.map_err(|_| std::io::Error::from(std::io::ErrorKind::Other));
        let writer_fut = writer.send_all(rx_adapter);

        //method 1, use reader_fut join writer_fut
        let joined = reader_fut.join(writer_fut);
        joined.map(|_| ()).map_err(|_| ())
    }

    fn filter_packet(bytes: BytesMut) -> Option<tcp::FilterPacket> {
        if let Ok(value) = PacketHeaders::from_ip_slice(&bytes[..]) {
            if let Some(IpHeader::Version4(ipv4)) = value.ip {
                match  ipv4.protocol {
                    6/*IpTrafficClass::Tcp*/  =>{
                        let tcph = value.transport.unwrap().tcp().unwrap();
                        if tcph.syn {
                            return Some(tcp::FilterPacket::TcpSyn(ipv4,tcph));
                        }
                        let data_begin = ipv4.header_len() as usize + tcph.header_len() as usize ;
                        let data_end = bytes.len();
                        let payload = &bytes[data_begin..data_end];
                        return Some(tcp::FilterPacket::Tcp(SocketAddrV4::new(Ipv4Addr::from(ipv4.source), tcph.source_port),
                                                           tcph,
                                                           Bytes::from(payload)));
                    },
                    1/*IpTrafficClass::Icmp*/ =>{
                        return Some(tcp::FilterPacket::Ping(bytes))
                    },
                    _/*others,ignore*/=>{}
                }
            }
        }
        None
    }

    fn gen_ping_echo(mut bytes: BytesMut) -> Bytes {
        //swap source ip and destination ip

        let src = bytes[12..].as_mut_ptr() as *mut [u8; 4];
        let dst = bytes[16..].as_mut_ptr() as *mut [u8; 4];
        unsafe {
            std::ptr::swap(src, dst);
        }

        //change to type=0,Echo Reply
        bytes[20] = 0u8;
        bytes.freeze()
    }
}

trait Default {
    fn default() -> Self;
}
impl Default for Device {
    fn default() -> Self {
        let mut config = tun::Configuration::default();

        config
            .address((10, 0, 0, 1))
            .netmask((255, 255, 255, 0))
            .up();

        #[cfg(target_os = "linux")]
        config.platform(|config| {
            config.packet_information(false);
        });

        let dev = Device::new(&config).unwrap();

        //set non-blocking mode for /dev/tun0
        unsafe {
            use std::os::unix::io::AsRawFd;
            let mut nonblock: libc::c_int = 1;
            libc::ioctl(dev.as_raw_fd(), libc::FIONBIO, &mut nonblock);
        }
        dev
    }
}
