// based on https://github.com/tokio-rs/tokio-core/blob/master/examples/echo-threads.rs

extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate tokio_multiple_cores;

use std::net::{self, SocketAddr};
use std::thread;

use futures::Stream;
use futures::sync::mpsc;
use hyper::server::Http;
use tokio_core::net::TcpStream;
use tokio_core::reactor::Core;
use tokio_multiple_cores::App;


fn main() {
    let addr: SocketAddr = ([127, 0, 0, 1], 4000).into();
    let worker_count = 3;

    // Use `std::net` to bind the requested port, we'll use this on the main
    // thread below
    let listener = net::TcpListener::bind(&addr).expect("failed to bind");
    println!("Listening on: {}", addr);

    // Spin up our worker threads, creating a channel routing to each worker
    // thread that we'll use below.
    let mut channels = Vec::new();
    for _ in 0..worker_count {
        let (tx, rx) = mpsc::unbounded();
        channels.push(tx);
        thread::spawn(|| worker(rx));
    }

    // Infinitely accept sockets from our `std::net::TcpListener`, as this'll do
    // blocking I/O. Each socket is then shipped round-robin to a particular
    // thread which will associate the socket with the corresponding event loop
    // and process the connection.
    let mut next = 0;
    for socket in listener.incoming() {
        let socket = socket.expect("failed to accept");
        channels[next].unbounded_send(socket).expect(
            "worker thread died",
        );
        next = (next + 1) % channels.len();
    }
}

fn worker(rx: futures::sync::mpsc::UnboundedReceiver<net::TcpStream>) {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let http = Http::new();

    let done = rx.for_each(move |socket| {
        // First up when we receive a socket we associate it with our event loop
        // using the `TcpStream::from_stream` API. After that the socket is not
        // a `tokio_core::net::TcpStream` meaning it's in nonblocking mode and
        // ready to be used with Tokio
        let socket =
            TcpStream::from_stream(socket, &handle).expect("failed to associate TCP stream");
        let addr = socket.peer_addr().expect("failed to get remote address");

        http.bind_connection(&handle, socket, addr, App::new(&handle));

        Ok(())
    });
    core.run(done).unwrap();
}
