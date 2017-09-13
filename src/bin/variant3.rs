extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate tokio_multiple_cores;

use std::thread;

use futures::Stream;
use futures::sync::mpsc;
use hyper::server::Http;
use tokio_core::net::{TcpListener, TcpStream};
use tokio_core::reactor::Core;
use tokio_multiple_cores::App;

fn main() {
    let addr = ([127, 0, 0, 1], 4000).into();
    let worker_count = 3;

    // Spin up our worker threads, creating a channel routing to each worker
    // thread that we'll use below.
    let mut channels = Vec::new();
    for _ in 0..worker_count {
        let (tx, rx) = mpsc::unbounded();
        channels.push(tx);
        thread::spawn(|| worker(rx));
    }

    // Ship each socket round-robin to a particular thread which will associate the socket with the
    // corresponding event loop and process the connection.
    let mut core = Core::new().expect("unable to initialize the main event loop");
    let listener = TcpListener::bind(&addr, &core.handle()).expect("unable to listen");
    println!(
        "Listening on http://{} with 1 thread. Processing requests with {} threads.",
        listener.local_addr().unwrap(),
        worker_count,
    );

    let mut next = 0;
    let srv = listener.incoming().for_each(move |(socket, _)| {
        channels[next].unbounded_send(socket).expect(
            "worker thread died",
        );
        next = (next + 1) % channels.len();

        Ok(())
    });

    core.run(srv).expect("error running the event loop");
}

fn worker(rx: futures::sync::mpsc::UnboundedReceiver<TcpStream>) {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let http = Http::new();

    let done = rx.for_each(move |socket| {
        let addr = socket.peer_addr().expect("failed to get remote address");

        http.bind_connection(&handle, socket, addr, App::new(&handle));

        Ok(())
    });
    core.run(done).unwrap();
}
