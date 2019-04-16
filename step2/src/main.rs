use futures::future::Future;
use futures::stream::Stream;
use std::time::{Duration, Instant};
use tokio::timer::Interval;

use libtcp_step2::Libtcp;

fn main() {
    let egine = Libtcp::process();
    let task = tick().join(egine).map(|_| ());    
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
