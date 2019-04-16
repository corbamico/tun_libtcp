use futures::future::Future;
use futures::stream::Stream;
use std::time::{Duration, Instant};
use tokio::timer::Interval;

use libtcp_step1::Libtcp;

fn main() {
    let egine = Libtcp::process();
    let task = main_task();
    let task = task.join(egine).map(|_| ());

    tokio::run(task);
}

fn main_task() -> impl Future<Item = (), Error = ()> {
    let now = Instant::now();
    let main_loop = Interval::new(now, Duration::from_secs(2))
        .for_each(move |v| {
            eprintln!("[main] tick: {:?}", v.duration_since(now));
            Ok(())
        })
        .map_err(|e| eprint!("[main] tick error:{:?}", e));

    main_loop
}
