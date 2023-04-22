//! An atomic register based on the implementation by Attiya, Bar-Noy, and
//! Dolev [[ABD95]](https://dl.acm.org/doi/pdf/10.1145/200836.200869).
//! use bytes::Bytes;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use bytes::{Buf, Bytes};
use http_body_util::{BodyExt, Full};
use hyper::service::Service;
use hyper::{body::Incoming as IncomingBody, Method, Request, Response};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

fn mk_response(
    s: String,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::builder().body(Full::new(Bytes::from(s))).unwrap())
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct LocalValue<T: Default + Ord + Send> {
    label: u32,
    value: T,
}

#[derive(Clone)]
pub struct AtomicRegister<T: Default + Ord + Send> {
    local: Arc<Mutex<LocalValue<T>>>,
}

impl<T: Default + Ord + Send> AtomicRegister<T> {
    pub fn new() -> Self {
        Self {
            local: Arc::new(Mutex::new(LocalValue::default())),
        }
    }
}

impl<T: Default + Ord + Send> Default for AtomicRegister<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Debug + Default + DeserializeOwned + Ord + Send + Serialize + 'static>
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

                let mut local = local.lock().unwrap();
                if other > *local {
                    *local = other
                };

                let serialized = serde_json::to_string(&*local);
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
mod tests {
    use super::*;

    mod local_value {
        use super::*;

        #[test]
        fn orders_by_label_first() {
            let first = LocalValue { label: 0, value: 1 };
            let second = LocalValue { label: 1, value: 0 };
            assert!(first < second)
        }

        #[test]
        fn orders_by_value_if_labels_match() {
            let first = LocalValue { label: 0, value: 0 };
            let second = LocalValue { label: 0, value: 1 };
            assert!(first < second)
        }
    }
}
