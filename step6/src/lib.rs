
#![allow(dead_code)]
use futures::stream::Stream;
use futures::{Future, Poll,Async};
use futures::sink::Sink;

use std::io;
use std::net::SocketAddr;
//use std::sync::mpsc;
use futures::sync::mpsc;

use tun::platform::Device;

use tokio::codec::{BytesCodec, FramedRead,FramedWrite};
use tokio::reactor::PollEvented2;
use tokio::runtime::Runtime;
use tokio::io::{AsyncRead};


use bytes::{BytesMut,Bytes};

use etherparse::*;

mod tcp;

#[derive(Debug)]
pub struct TcpListener;

#[derive(Debug)]
pub struct Incoming;

#[derive(Debug)]
pub struct TcpStream;

impl TcpListener {
    pub fn bind(_addr: &SocketAddr) -> io::Result<Self> {
        Ok(TcpListener{})
        //unimplemented!()
    }
    pub fn incoming(self) -> Incoming {
        Incoming{}
        //unimplemented!()
    }
}

impl Stream for Incoming {
    type Item = TcpStream;
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        Ok(Async::NotReady)
        //unimplemented!()
    }
}


///====== tcp protocol engine ======
pub struct Libtcp;

impl Libtcp {
    pub fn process_in_runtime(rt: &mut Runtime) -> io::Result<()> {
        let task = Self::process();
        rt.spawn(task);
        Ok(())
    }
    pub fn process() -> impl Future<Item = (), Error = ()> {
        let event = PollEvented2::new(Device::default());        
        let (rd,wr) = event.split();
        let writer = FramedWrite::new(wr,BytesCodec::new());

        let (tx,rx) = mpsc::channel(1500);
        //let (tx,rx) = mpsc::channel();

        let read_fut = FramedRead::new(rd, BytesCodec::new())
            .filter_map(|bytes| {
                
                let tcpcon = Self::filter_with_tcp(&bytes).map(tcp::FilterPacket::Tcp);
                let icmpbytes = Self::filter_with_icmp(bytes).map(tcp::FilterPacket::Ping);
                tcpcon.or(icmpbytes) 
            })
            //Step 2: handle icmp packet            
            .map(move |packet|{

                match packet{
                    tcp::FilterPacket::Ping(bytes)=>{
                        let echo_bytes = Self::gen_ping_echo(bytes);
                        tx.clone().send(echo_bytes).wait().unwrap();
                    }
                    tcp::FilterPacket::Tcp(_conn)=>{

                    }
                }

                ()
            })
            .collect();

        let rx_adapter = rx.map_err(|_|std::io::Error::from(std::io::ErrorKind::Other));
        let writer_fut = writer.send_all(rx_adapter);

        //method 1, use reader_fut join writer_fut
        let joined = read_fut.join(writer_fut);
        joined.map(|_|()).map_err(|_|())

        //method 2, spawn writer in another thread 
        // let writer_fut = futures::future::lazy(||{
        //     tokio::spawn(writer_fut.map(|_|()).map_err(|_|()));
        //     Ok(())
        // });
        // let joined = read_fut.join(writer_fut);
        // joined.map(|_|()).map_err(|_|())
 
    }

    fn filter_with_tcp(bytes: &BytesMut)->Option<tcp::Connection>{
        if let Ok(value) = PacketHeaders::from_ip_slice(&bytes[..]) {
            if let Some(IpHeader::Version4(ipv4)) = value.ip {
                if IpTrafficClass::Tcp as u8 == ipv4.protocol {
                    let tcph = value.transport.unwrap().tcp().unwrap();

                    if tcph.syn || tcph.fin || tcph.psh {
                        return Some(tcp::Connection::new(&ipv4,&tcph))
                    }
                }                    
            }
        }
        None
    }
    fn gen_tcp_packet(mut con: tcp::Connection)->Bytes{
        
        let buf = "hello\n".as_bytes();

        if !con.tcph.psh {
            con.gen_packet(&[0;0])
        }else{
            con.gen_packet(buf)
        }
    }

    fn filter_with_icmp(bytes: BytesMut)->Option<BytesMut>{
        if let Ok(value) = PacketHeaders::from_ip_slice(&bytes[..]) {
            if let Some(IpHeader::Version4(ipv4)) = value.ip {                
                //we only care about icmp packet and type=8 Echo Request
                if IpTrafficClass::Icmp as u8 == ipv4.protocol
                    && bytes[20]== 8 {                            
                    return Some(bytes)
                }                        
            }
        }
        Option::None
    }
    fn gen_ping_echo(mut bytes: BytesMut)->Bytes{
        //swap source ip and destination ip        

        let src = bytes[12..].as_mut_ptr() as * mut [u8;4];
        let dst = bytes[16..].as_mut_ptr() as * mut [u8;4];
        unsafe {std::ptr::swap(src, dst);}

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
