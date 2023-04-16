//! An atomic register based on the implementation by Attiya, Bar-Noy, and
//! Dolev [[ABD95]](https://dl.acm.org/doi/pdf/10.1145/200836.200869).
//! use bytes::Bytes;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use http_body_util::Full;
use hyper::service::Service;
use hyper::{body::Incoming as IncomingBody, Request, Response};
use serde::Serialize;

type RegisterResponse = Response<Full<Bytes>>;
type GenericError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Default, Serialize)]
struct LocalValue<T: Default> {
    label: u32,
    value: T,
}

pub struct AtomicRegister<T: Default> {
    local: LocalValue<T>,
}

impl<T: Default> AtomicRegister<T> {
    pub fn new() -> Self {
        Self {
            local: LocalValue::default(),
        }
    }
}

impl<T: Debug + Default + Serialize> Service<Request<IncomingBody>> for AtomicRegister<T> {
    type Response = Response<Full<Bytes>>; // RegisterResponse
    type Error = Box<dyn std::error::Error + Send + Sync>; // GenericError
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&mut self, req: Request<IncomingBody>) -> Self::Future {
        fn mk_response(
            s: String,
        ) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
            Ok(Response::builder().body(Full::new(Bytes::from(s))).unwrap())
        }

        let res = match req.uri().path() {
            "/register/local" => {
                let serialized = serde_json::to_string(&self.local);
                match serialized {
                    Ok(value) => mk_response(value),
                    Err(err) => return Box::pin(async { Err(err.into()) }),
                }
            }
            // Return the 404 Not Found for other routes, and don't increment counter.
            _ => return Box::pin(async { mk_response("oh no! not found".into()) }),
        };

        Box::pin(async { res })
    }
}

#[cfg(test)]
mod tests {}
