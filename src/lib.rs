extern crate futures;
extern crate hyper;
extern crate tokio_core;

use futures::{future, Future};
use hyper::server::{Service, Request, Response};
use tokio_core::reactor::Handle;

pub struct App {
    #[allow(dead_code)]
    handle: Handle,
}

impl App {
    pub fn new(handle: &Handle) -> Self {
        App { handle: handle.clone() }
    }
}

impl Service for App {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, _: Self::Request) -> Self::Future {
        Box::new(future::ok(Response::default().with_body("Works!")))

        // let sleep = Timeout::new(Duration::from_millis(10), &self.handle).unwrap();
        // Box::new(sleep.map_err(|_| unimplemented!()).map(move |_| {
        //     Response::default().with_body("Works!")
        // }))
    }
}
