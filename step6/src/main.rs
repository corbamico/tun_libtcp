use futures::future::{Future};
use futures::stream::Stream;
use std::time::{Duration, Instant};
use tokio::timer::Interval;

use libtcp_step6::{Libtcp,TcpListener};

fn main() {
    let egine = Libtcp::instance().process();
    let demo_tcp_server = tcp_srv();

    let task = tick().join3(egine,demo_tcp_server).map(|_| ());    

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

fn tcp_srv()->impl Future<Item = (), Error = ()> {
    let addr = "127.0.0.1:8080".parse().unwrap();
    let listener = TcpListener::bind(&addr).unwrap();
    let server = listener.incoming().for_each(|_stream|{
        //use stream
        println!("we got a client, aha!");
        Ok(())
    })
    .map_err(|e|{
        eprintln!("accept error = {:?}",e);
    });

    server
}