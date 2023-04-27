//! An atomic register based on the implementation by Attiya, Bar-Noy, and
//! Dolev [[ABD95]](https://dl.acm.org/doi/pdf/10.1145/200836.200869).
//! use bytes::Bytes;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use bytes::{Buf, Bytes};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::Incoming;
use hyper::service::Service;
use hyper::{Method, Request, Response, Uri};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

use crate::net::TcpStream;

type GenericError = Box<dyn std::error::Error + Send + Sync>;

fn mk_response(
    s: String,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::builder().body(Full::new(Bytes::from(s))).unwrap())
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct LocalValue<T: Clone + Default + Ord + Send> {
    label: u32,
    value: T,
}

#[derive(Clone)]
pub struct AtomicRegister<T: Clone + Default + DeserializeOwned + Ord + Send> {
    neighbors: Vec<Uri>,
    local: Arc<Mutex<LocalValue<T>>>,
}

impl<T: Clone + Default + DeserializeOwned + Ord + Send + 'static> Default for AtomicRegister<T> {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl<T: Clone + Default + DeserializeOwned + Ord + Send + 'static> AtomicRegister<T> {
    pub fn new(neighbors: Vec<Uri>) -> Self {
        Self {
            neighbors,
            local: Arc::new(Mutex::new(LocalValue::default())),
        }
    }

    async fn communicate(
        local: LocalValue<T>,
        neighbors: Vec<Uri>,
    ) -> Result<Vec<Option<LocalValue<T>>>, GenericError> {
        let mut results: Vec<Option<LocalValue<T>>> = vec![Some(local)];
        results.resize_with(neighbors.len() + 1, Default::default);

        let info = Arc::new(Mutex::new(results));
        let mut handles = JoinSet::new();

        let majority = (neighbors.len() as f32 / 2.0).ceil() as u32;
        for (i, neighbor) in neighbors.into_iter().enumerate() {
            let info = info.clone();
            handles.spawn(async move {
                // TODO: Refactor this to be better...
                let mut parts = neighbor.into_parts();
                parts.path_and_query = Some("/register/local".parse().unwrap());
                let addr = Uri::from_parts(parts)?;

                let authority = addr.authority().ok_or("Invalid URL")?.as_str();

                let stream = TcpStream::connect(authority).await?;
                let (mut sender, conn) = hyper::client::conn::http1::handshake(stream).await?;
                tokio::task::spawn(async move {
                    if let Err(err) = conn.await {
                        println!("Connection failed: {err}");
                    }
                });

                let authority = addr.authority().unwrap().clone();

                let req = Request::builder()
                    .uri(addr)
                    .method(Method::GET)
                    .header(hyper::header::HOST, authority.as_str())
                    .body(empty())?;

                let res = sender.send_request(req).await?;
                let body = res.collect().await?;
                let value: LocalValue<T> = serde_json::from_reader(body.aggregate().reader())?;

                let mut data = info.lock().unwrap();
                (*data)[i] = Some(value);
                Ok::<(), GenericError>(())
            });
        }

        let mut acks = 0;
        while acks < majority {
            if handles.join_next().await.is_some() {
                acks += 1;
            }
        }

        let results = info.lock().unwrap().clone();
        Ok(results)
    }
}

impl<T: Clone + Debug + Default + DeserializeOwned + Ord + Send + Serialize + 'static>
    Service<Request<Incoming>> for AtomicRegister<T>
{
    type Response = Response<Full<Bytes>>;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&mut self, req: Request<Incoming>) -> Self::Future {
        let local = self.local.clone();
        let neighbors = self.neighbors.clone();
        match (req.method(), req.uri().path()) {
            // GET requests perform a 'read' on the shared-register.
            (&Method::GET, "/register") => Box::pin(async move {
                // This inner-block is required for the compiler to understand
                // that the lock is not required accross the call to .await.
                // See: https://github.com/rust-lang/rust/issues/104883
                let local = {
                    let locked_local = local.lock().unwrap();
                    locked_local.clone()
                };
                let info = Self::communicate(local, neighbors).await?;
                let max = info.into_iter().max().unwrap().unwrap();
                let raw_value = serde_json::to_string(&max.value)?;
                mk_response(raw_value)
            }),
            // GET requests return this severs local value and associated label
            (&Method::GET, "/register/local") => Box::pin(async move {
                let value = serde_json::to_string(&local)?;
                println!("Responding!"); // TODO: Why doesn't this print?
                mk_response(value)
            }),
            // POST requests take another value and label as input, updates
            // this servers local value to be the _greater_ of the two, and
            // returns it, along with the associated label.
            (&Method::POST, "/register/local") => Box::pin(async move {
                let body = req.collect().await?.aggregate();
                let other: LocalValue<T> = serde_json::from_reader(body.reader())?;

                let mut local = local.lock().unwrap();
                if other > *local {
                    *local = other
                };

                let value = serde_json::to_string(&*local)?;
                mk_response(value)
            }),
            // Return the 404 Not Found for other routes, and don't increment counter.
            // TODO: Improve this...
            _ => Box::pin(async { mk_response("404 Not Found".into()) }),
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
