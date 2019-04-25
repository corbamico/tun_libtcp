use futures::future::Future;
use futures::stream::Stream;
use std::time::{Duration, Instant};
use tokio::timer::Interval;

use libtcp_step7::{Libtcp, TcpListener};

fn main() {
    let mut libtcp = Libtcp::instance();
    let demo_tcp_server = tcp_srv(&mut libtcp);
    let egine = libtcp.process();
    let task = tick().join3(egine, demo_tcp_server).map(|_| ());
    tokio::run(task);
}

fn tick() -> impl Future<Item = (), Error = ()> {
    let now = Instant::now();

    Interval::new(now, Duration::from_secs(1))
        .for_each(move |v| {
            eprint!("\r[main] tick: {:?}  ", v.duration_since(now));
            Ok(())
        })
        .map_err(|e| eprint!("[main] tick error:{:?}", e))
}

fn tcp_srv(lib: &mut Libtcp) -> impl Future<Item = (), Error = ()> {
    let addr = "127.0.0.1:8080".parse().unwrap();
    let listener = TcpListener::bind(lib, &addr).unwrap();

    let server = listener.incoming().for_each(|stream| {
        println!("\n[tcp demo server]accept new client");
        let task = stream.for_each(|bytes| {
            println!(
                "\n[tcp demo server]recieve:{:?}",
                String::from_utf8_lossy(&bytes[..])
            );
            Ok(())
        });
        tokio::spawn(task);
        Ok(())
    });
    server
}
