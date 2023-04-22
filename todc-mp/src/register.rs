//! An atomic register based on the implementation by Attiya, Bar-Noy, and
//! Dolev [[ABD95]](https://dl.acm.org/doi/pdf/10.1145/200836.200869).
//! use bytes::Bytes;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};

use bytes::{Buf, Bytes};
use http_body_util::{BodyExt, Full};
use hyper::service::Service;
use hyper::{body::Incoming as IncomingBody, Method, Request, Response};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

type RegisterResponse = Response<Full<Bytes>>;
type GenericError = Box<dyn std::error::Error + Send + Sync>;

fn mk_response(
    s: String,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::builder().body(Full::new(Bytes::from(s))).unwrap())
}

#[derive(Default, Deserialize, Serialize)]
struct LocalValue<T: Default + Send> {
    label: AtomicU32,
    value: Mutex<T>,
}

pub struct AtomicRegister<T: Default + Send> {
    local: Arc<LocalValue<T>>,
}

impl<T: Default + Send> AtomicRegister<T> {
    pub fn new() -> Self {
        Self {
            local: Arc::new(LocalValue::default()),
        }
    }
}

impl<T: Debug + Default + DeserializeOwned + Send + Serialize + 'static>
    Service<Request<IncomingBody>> for AtomicRegister<T>
{
    type Response = Response<Full<Bytes>>; // RegisterResponse
    type Error = Box<dyn std::error::Error + Send + Sync>; // GenericError
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&mut self, req: Request<IncomingBody>) -> Self::Future {
        let local = self.local.clone();
        match (req.method(), req.uri().path()) {
            (&Method::GET, "/register/local") => Box::pin(async move {
                let serialized = serde_json::to_string(&local);
                match serialized {
                    Ok(value) => mk_response(value),
                    Err(err) => Err(err.into()),
                }
            }),
            (&Method::POST, "/register/local") => Box::pin(async move {
                let body = req.collect().await?.aggregate();
                let other: LocalValue<T> = serde_json::from_reader(body.reader())?;
                let serialized = serde_json::to_string(&other);
                match serialized {
                    Ok(value) => mk_response(value),
                    Err(err) => Err(err.into()),
                }
            }),
            // Return the 404 Not Found for other routes, and don't increment counter.
            _ => Box::pin(async { mk_response("oh no! not found".into()) }),
        }
    }
}

#[cfg(test)]
mod tests {}
