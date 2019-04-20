
#![allow(dead_code)]

use futures::stream::Stream;
use futures::{Future, Poll};

use std::io;
use std::net::SocketAddr;

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
        unimplemented!()
    }
    pub fn incoming(self) -> Incoming {
        unimplemented!()
    }
}

impl Stream for Incoming {
    type Item = TcpStream;
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        unimplemented!()
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

        FramedRead::new(rd, BytesCodec::new())
            .filter_map(|bytes| Self::filter_with_tcp(bytes))            
            .map(|con| Self::gen_tcp_packet(con))
            .forward(writer)
            .map(|_|())
            .map_err(|e| eprintln!("[libtcp] error:{:?}", e))
    }

    fn filter_with_tcp(bytes: BytesMut)->Option<tcp::Connection>{
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
