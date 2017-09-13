extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate tokio_multiple_cores;

use std::thread;

use futures::Stream;
use hyper::server::Http;
use tokio_core::net::TcpListener;
use tokio_core::reactor::{Core, Remote};
use tokio_multiple_cores::App;


fn main() {
    use std::sync::mpsc;

    let mut threads = Vec::new();

    let worker_count = 3;
    let (tx, rx) = mpsc::channel();
    for _ in 0..worker_count {
        let tx = tx.clone();
        threads.push(thread::spawn(move || {
            let mut core = Core::new().expect("unable to initialize worker event loop");

            tx.send(core.remote()).expect("Channel was closed early");

            loop {
                core.turn(None);
            }
        }));
    }

    let remotes: Vec<Remote> = rx.into_iter().take(worker_count).collect();

    let mut core = Core::new().expect("unable to initialize the main event loop");
    let addr = ([127, 0, 0, 1], 4000).into();
    let listener = TcpListener::bind(&addr, &core.handle()).expect("unable to listen");
    println!(
        "Listening on http://{} with 1 thread. Processing requests with {} threads.",
        listener.local_addr().unwrap(),
        worker_count,
    );

    let http = Http::new();
    let mut next_ix = 0;
    let srv = listener.incoming().for_each(move |(socket, addr)| {
        let remote = remotes.get(next_ix).unwrap();
        next_ix = (next_ix + 1) % remotes.len();
        let http = http.clone();

        remote.spawn(move |handle| {
            http.bind_connection(handle, socket, addr, App::new(&handle));
            Ok(())
        });
        Ok(())
    });

    core.run(srv).expect("error running the event loop");

    for thread in threads {
        thread.join().expect("Worker thread paniced");
    }
}
