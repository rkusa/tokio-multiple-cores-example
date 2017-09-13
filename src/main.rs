extern crate futures;
extern crate futures_cpupool;
extern crate hyper;
extern crate tokio_core;

use hyper::server::Service;
use futures::{future, Future, Stream};
use hyper::server::Http;
use tokio_core::net::TcpListener;
use tokio_core::reactor::Core;
use std::sync::mpsc;
use std::thread;
use tokio_core::reactor::Remote;
use hyper::{Request, Response};

struct App;

impl Service for App {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, _: Self::Request) -> Self::Future {
        Box::new(future::ok(Response::default().with_body("Works!")))
    }
}

fn main() {
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
        let ix = next_ix % remotes.len();
        next_ix += 1;
        if next_ix == remotes.len() {
            next_ix = 0;
        }

        let remote = remotes.get(ix).unwrap();
        let http = http.clone();

        remote.spawn(move |handle| {
            http.bind_connection(
                handle,
                socket,
                addr,
                App{},
            );
            Ok(())
        });
        Ok(())
    });

    core.run(srv).expect("error running the event loop");

    for thread in threads {
        thread.join().expect("Worker thread paniced");
    }
}

