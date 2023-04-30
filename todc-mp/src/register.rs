//! An atomic register based on the implementation by Attiya, Bar-Noy, and
//! Dolev [[ABD95]](https://dl.acm.org/doi/pdf/10.1145/200836.200869).
//! use bytes::Bytes;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use bytes::{Buf, Bytes};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::Service;
use hyper::{Method, Request, Response, Uri};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value as JSON;
use tokio::task::JoinSet;

use crate::{get, post, GenericError};

fn mk_response(
    value: JSON,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::builder()
        .body(Full::new(Bytes::from(value.to_string())))
        .unwrap())
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct LocalValue<T: Clone + Debug + Default + Ord + Send> {
    label: u32,
    value: T,
}

#[derive(Clone)]
pub struct AtomicRegister<T: Clone + Debug + Default + DeserializeOwned + Ord + Send> {
    neighbors: Vec<Uri>,
    local: Arc<Mutex<LocalValue<T>>>,
}

impl<T: Clone + Debug + Default + DeserializeOwned + Ord + Send + Serialize + 'static> Default
    for AtomicRegister<T>
{
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

#[derive(Clone, Copy)]
enum MessageType {
    /// A message _announcing_ the senders value and label, with the intention of
    /// having recievers adopt the value if its label is larger than than theirs.
    Announce,
    /// A message _asking_ for the recievers value and label.
    Ask,
}

impl<T: Clone + Debug + Default + DeserializeOwned + Ord + Send + Serialize + 'static>
    AtomicRegister<T>
{
    pub fn new(neighbors: Vec<Uri>) -> Self {
        Self {
            neighbors,
            local: Arc::new(Mutex::new(LocalValue::default())),
        }
    }

    async fn communicate(
        &self,
        message: MessageType,
    ) -> Result<Vec<Option<LocalValue<T>>>, GenericError> {
        let local = self.local.lock().unwrap().clone();
        let mut results: Vec<Option<LocalValue<T>>> = vec![Some(local.clone())];
        results.resize_with(self.neighbors.len() + 1, Default::default);

        let mut handles = JoinSet::new();
        let info = Arc::new(Mutex::new(results));
        let majority = (self.neighbors.len() as f32 / 2.0).ceil() as u32;

        for (i, url) in self.neighbor_urls().into_iter().enumerate() {
            let info = info.clone();
            let local = local.clone();
            handles.spawn(async move {
                let res = match message {
                    MessageType::Announce => {
                        let body = serde_json::to_value(local)?;
                        post(url, body).await?
                    }
                    MessageType::Ask => get(url).await?,
                };
                let body = res.collect().await?.aggregate();
                let value: LocalValue<T> = serde_json::from_reader(body.reader())?;

                let mut info = info.lock().unwrap();
                (*info)[i + 1] = Some(value);
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

    pub fn neighbor_urls(&self) -> Vec<Uri> {
        let neighbors = self.neighbors.clone();
        neighbors
            .into_iter()
            .map(|addr| {
                let mut parts = addr.into_parts();
                parts.path_and_query = Some("/register/local".parse().unwrap());
                Uri::from_parts(parts).unwrap()
            })
            .collect()
    }

    async fn read(&self) -> Result<T, GenericError> {
        let info = self.communicate(MessageType::Ask).await?;
        let max = info.into_iter().max().unwrap().unwrap();
        let local = self.update(&max);
        self.communicate(MessageType::Announce).await?;
        Ok(local.value)
    }

    fn update(&self, other: &LocalValue<T>) -> LocalValue<T> {
        let mut local = self.local.lock().unwrap();
        if *other > *local {
            *local = other.clone()
        };
        local.clone()
    }
}

impl<T: Clone + Debug + Default + DeserializeOwned + Ord + Send + Serialize + 'static>
    Service<Request<Incoming>> for AtomicRegister<T>
{
    type Response = Response<Full<Bytes>>;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&mut self, req: Request<Incoming>) -> Self::Future {
        // TODO: Explain this.
        let me = self.clone();
        match (req.method(), req.uri().path()) {
            // GET requests perform a 'read' on the shared-register.
            (&Method::GET, "/register") => Box::pin(async move {
                let value = me.read().await?;
                mk_response(serde_json::to_value(value)?)
            }),
            // GET requests return this severs local value and associated label
            (&Method::GET, "/register/local") => {
                Box::pin(async move { mk_response(serde_json::to_value(&me.local)?) })
            }
            // POST requests take another value and label as input, updates
            // this servers local value to be the _greater_ of the two, and
            // returns it, along with the associated label.
            (&Method::POST, "/register/local") => Box::pin(async move {
                let body = req.collect().await?.aggregate();
                let other: LocalValue<T> = serde_json::from_reader(body.reader())?;
                let local = me.update(&other);

                mk_response(serde_json::to_value(&local)?)
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
