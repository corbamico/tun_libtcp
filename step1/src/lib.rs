use futures::stream::Stream;
use futures::{Future, Poll};

use std::io;
use std::net::SocketAddr;

use tun::platform::Device;

use tokio::codec::{BytesCodec, FramedRead};
use tokio::reactor::PollEvented2;
use tokio::runtime::Runtime;

use etherparse::{PacketHeaders};

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
        let process = FramedRead::new(event, BytesCodec::new())
            .for_each(|bytes| {
                eprintln!("[libtcp] packet :{:?}", &bytes[..]);

                if let Ok(value) = PacketHeaders::from_ip_slice(&bytes[..]) {
                    Self::handle_ip_header(&value);
                }

                Ok(())
            })
            .map_err(|e| {
                eprintln!("[libtcp] error:{:?}", e);
            });
        process
    }

    fn handle_ip_header(value: &PacketHeaders) {
        println!("[libtcp] link: {:?}", value.link);
        println!("[libtcp] vlan: {:?}", value.vlan);
        println!("[libtcp]   ip: {:?}", value.ip);
        println!("[libtcp] tran: {:?}", value.transport);
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
