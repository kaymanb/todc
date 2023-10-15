//! Simulations of [atomic](https://en.wikipedia.org/wiki/Atomic_semantics)
//! [shared-memory registers](https://en.wikipedia.org/wiki/Shared_register)
//! as described by Attiya, Bar-Noy and Dolev
//! [\[ABD95\]](https://dl.acm.org/doi/pdf/10.1145/200836.200869).
//!
//! The atomicity guarantee only holds if at most a minority of instances
//! crash.
//!
//! # Examples
//!
//! In the following example, we create a single instance of the register that
//! will expose read and write operations as HTTP requests to `/register`. For
//! this example, our register will hold a type `String`.
//!
//! We can use [`hyper`] to run an instance of the register as follows:
//!
//! ```no_run
//! use std::net::SocketAddr;
//!
//! use http_body_util::{BodyExt, Full};
//! use hyper::{Method, Request, Response};
//! use hyper::body::{Bytes, Incoming};
//! use hyper::server::conn::http1;
//! use hyper::service::{Service, service_fn};
//! use hyper_util::rt::TokioIo;
//! use tokio::net::TcpListener;
//!
//! use todc_net::register::AtomicRegister;
//!
//! // The contents of the register
//! type Contents = String;
//!
//! // The main router for our server
//! async fn router(
//!     register: AtomicRegister<Contents>,
//!     req: Request<Incoming>
//! ) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
//!     match (req.method(), req.uri().path()) {
//!         // Allow the register to be read with GET requests
//!         (&Method::GET, "/register") => {
//!             let value: String = register.read().await.unwrap();
//!             Ok(Response::new(Full::new(Bytes::from(value))))
//!         },
//!         // Allow the register to be written to with POST requests
//!         (&Method::POST, "/register") => {
//!             let body = req.collect().await?.to_bytes();
//!             let value = String::from_utf8(body.to_vec()).unwrap();
//!             register.write(value).await.unwrap();
//!             Ok(Response::new(Full::new(Bytes::new())))
//!         },
//!         // Allow the register to handle all other requests, such as
//!         // internal requests made to /register/local.
//!         _ => register.call(req).await
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!    
//!     // Create a register for this instance.
//!     let register: AtomicRegister<Contents> = AtomicRegister::default();
//!
//!     // Create a new server with Hyper.
//!     let addr: SocketAddr = ([0, 0, 0, 0], 3000).into();
//!     let listener = TcpListener::bind(addr).await?;
//!     loop {
//!         let (stream, _) = listener.accept().await?;
//!         let io = TokioIo::new(stream);
//!         let register = register.clone();
//!         tokio::task::spawn(async move {
//!             if let Err(err) = http1::Builder::new()
//!                 // Handle requests by passing them to the router
//!                 .serve_connection(io, service_fn(move |req| router(register.clone(), req)))
//!                 .await
//!             {
//!                 println!("Error serving connection: {:?}", err)
//!             }
//!         });
//!     }    
//! }
//! ```
//!
//! ### Interacting with a Register
//!
//! Although this register isn't fault-tolerant yet, we can still try it out. See
//! the runnable example at [`todc-net/examples/atomic-register-hyper`](https://github.com/kaymanb/todc/tree/main/todc-net/examples/atomic-register-hyper).
//!
//! ## Adding Fault Tolerance with Multiple Instances
//!
//! To make our register fault tolerant, we need to add more instances. Suppose that
//! we want `3` instances, so that even if one instance fails the register will
//! continue to be available.
//!
//! If we have configured our infrastructure so that for each `i` in `[1, 2, 3]` the
//! server hosting instance `i` will be available at `https://my-register-{i}.com`, and
//! we have exposed `i` as an environmental variable `INSTANCE_ORDINAL`, then we
//! can instantiate the [`AtomicRegister`] as follows:
//!
//! ```no_run
//! use std::env;
//! # use hyper::Uri;
//! # use todc_net::register::AtomicRegister;
//! # type Contents = String;
//! // Replacement for `let register = AtomicRegister::default();`
//! let instance_ordinal: u32 = env::var("INSTANCE_ORDINAL").unwrap().parse().unwrap();
//! let neighbor_urls: Vec<Uri> = (1..4)
//!     .filter(|&i| i != instance_ordinal)
//!     .map(|i| format!("https://my-register-{i}.com").parse().unwrap())
//!     .collect();
//! let register: AtomicRegister<Contents> = AtomicRegister::new(neighbor_urls);
//! ```
//!
//! ### Interacting with a Fault Tolerant Register
//!
//! To interact with a fault-tolerant register backed by multiple instances, see
//! the runnable example at
//! [`todc-net/examples/atomic-register-docker-minikube`](https://github.com/kaymanb/todc/tree/main/todc-net/examples/atomic-register-docker-minikube).
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use bytes::{Buf, Bytes};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::http::StatusCode;
use hyper::service::Service;
use hyper::{Method, Request, Response, Uri};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

use crate::{get, mk_response, post, GenericError};

/// The local value of a register.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct LocalValue<T: Clone + Debug + Default + Ord + Send> {
    label: u32,
    value: T,
}

/// An [atomic](https://en.wikipedia.org/wiki/Atomic_semantics)
/// [shared-memory register](https://en.wikipedia.org/wiki/Shared_register).
///    
/// See the [`abd_95`](crate::register::abd_95) module-level documentation for
/// more details.
#[derive(Clone)]
pub struct AtomicRegister<T: Clone + Debug + Default + DeserializeOwned + Ord + Send> {
    neighbors: Vec<Uri>,
    local: Arc<Mutex<LocalValue<T>>>,
}

impl<T: Clone + Debug + Default + DeserializeOwned + Ord + Send + Serialize + 'static> Default
    for AtomicRegister<T>
{
    /// Creates an [`AtomicRegister`] with no neighbors.
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

/// A message from one register instance to another.
#[derive(Clone, Copy)]
enum Message {
    /// A message _announcing_ the senders value and label, with the intention of
    /// having recievers adopt the value if its label is larger than than theirs.
    Announce,
    /// A message _asking_ for the recievers value and label.
    Ask,
}

impl<T: Clone + Debug + Default + DeserializeOwned + Ord + Send + Serialize + 'static>
    AtomicRegister<T>
{
    /// Creates a new atomic register instance with a given set of neighbors.
    ///
    /// If there are `n` instances (servers) of [`AtomicRegister`], then
    /// each instance must be instantiated with a URL for all `n - 1` of
    /// it's neighbors.
    ///
    /// # Examples
    ///    
    /// Suppose that we want to create a network with 3 instances of [`AtomicRegister`],
    /// where each instance `i` is available at `https://my-register-{i}.com`. Then,
    /// we could instantiate instance `1` as follows:
    ///
    /// ```
    /// use std::env;
    /// use hyper::Uri;
    /// use todc_net::register::AtomicRegister;
    ///
    /// type Contents = String;
    ///
    /// let neighbor_urls: Vec<Uri> = (1..3)
    ///     .map(|i| format!("https://my-register-{i}").parse().unwrap())
    ///     .collect();
    ///
    /// let register: AtomicRegister<Contents> = AtomicRegister::new(neighbor_urls);
    /// ```
    pub fn new(neighbors: Vec<Uri>) -> Self {
        Self {
            neighbors,
            local: Arc::new(Mutex::new(LocalValue::default())),
        }
    }

    /// Sends and recieves a message from neighbors.
    async fn communicate(&self, message: Message) -> Result<Vec<LocalValue<T>>, GenericError> {
        let local = self.local.lock().unwrap().clone();

        // Communicate the message with all neighbors.
        let mut handles = JoinSet::new();
        for url in self.neighbor_urls().into_iter() {
            let local = local.clone();
            handles.spawn(async move {
                let result = match message {
                    Message::Announce => {
                        let body = serde_json::to_value(local)?;
                        post(url, body).await
                    }
                    Message::Ask => get(url).await,
                };

                match result {
                    Err(error) => Err(error),
                    Ok(response) => {
                        if response.status().is_server_error() {
                            return Err(GenericError::from("Unexpected server error"));
                        }

                        let body = response.collect().await?.aggregate();
                        let value: LocalValue<T> = serde_json::from_reader(body.reader())?;
                        Ok(value)
                    }
                }
            });
        }

        // Wait until a majority of neighbors have replied succesfully, and
        // return their values.
        let mut info: Vec<LocalValue<T>> = vec![local.clone()];

        let mut acks: f32 = 1.0;
        let mut failures: f32 = 0.0;
        let minority = (self.neighbors.len() as f32 + 1_f32) / 2_f32;
        while acks <= minority && failures <= minority {
            if let Some(result) = handles.join_next().await {
                match result? {
                    Err(_) => failures += 1.0,
                    Ok(value) => {
                        info.push(value);
                        acks += 1.0;
                    }
                }
            }
        }

        if acks > minority {
            Ok(info)
        } else {
            Err(GenericError::from("A majority of neighbors are offline"))
        }
    }

    /// Returns a set of URLs that neighboring instances can be reached at.
    fn neighbor_urls(&self) -> Vec<Uri> {
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

    /// Returns the value contained in the register.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tokio_test;
    /// use todc_net::register::AtomicRegister;
    ///
    /// type Contents = u32;
    /// # tokio_test::block_on(async {
    /// let register: AtomicRegister<Contents> = AtomicRegister::default();
    /// assert_eq!(register.read().await.unwrap(), 0);
    /// # })
    /// ```
    pub async fn read(&self) -> Result<T, GenericError> {
        let info = self.communicate(Message::Ask).await?;
        let max = info.into_iter().max().unwrap();
        let local = self.update(&max);
        self.communicate(Message::Announce).await?;
        Ok(local.value)
    }

    /// Updates the local value of this register instance.
    fn update(&self, other: &LocalValue<T>) -> LocalValue<T> {
        let mut local = self.local.lock().unwrap();
        if *other > *local {
            *local = other.clone()
        };
        local.clone()
    }

    /// Sets the contents of the register to the specified value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tokio_test;
    /// use todc_net::register::AtomicRegister;
    ///
    /// type Contents = u32;
    ///
    /// # tokio_test::block_on(async {
    /// let register: AtomicRegister<Contents> = AtomicRegister::default();
    /// register.write(123).await;
    /// assert_eq!(register.read().await.unwrap(), 123);
    /// # })
    /// ```
    pub async fn write(&self, value: T) -> Result<(), GenericError> {
        let new = LocalValue {
            value,
            label: self.local.lock().unwrap().label + 1,
        };
        self.update(&new);
        self.communicate(Message::Announce).await?;
        Ok(())
    }
}

impl<T: Clone + Debug + Default + DeserializeOwned + Ord + Send + Serialize + 'static>
    Service<Request<Incoming>> for AtomicRegister<T>
{
    type Response = Response<Full<Bytes>>;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        // The Future we return can be send to other tasks or threads and
        // need to make sure that the objects it references remain valid:
        // https://rust-lang.github.io/async-book/03_async_await/01_chapter.html#async-lifetimes
        //
        // One option is to clone indivdual fields and pass them into static
        // methods, but `let me = self.clone()` provides a much cleaner API.
        // https://www.philipdaniels.com/blog/2020/self-cloning-for-multiple-threads-in-rust/
        let me = self.clone();
        match (req.method(), req.uri().path()) {
            // GET requests return this severs local value and associated label
            (&Method::GET, "/register/local") => {
                Box::pin(
                    async move { mk_response(StatusCode::OK, serde_json::to_value(&me.local)?) },
                )
            }
            // POST requests take another value and label as input, updates
            // this servers local value to be the _greater_ of the two, and
            // returns it, along with the associated label.
            (&Method::POST, "/register/local") => Box::pin(async move {
                let body = req.collect().await?.aggregate();
                let other: LocalValue<T> = serde_json::from_reader(body.reader())?;
                let local = me.update(&other);
                mk_response(StatusCode::OK, serde_json::to_value(&local)?)
            }),
            _ => Box::pin(async { mk_response(StatusCode::NOT_FOUND, "404 Not Found".into()) }),
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

    mod atomic_register {
        use super::*;

        mod communicate {
            use super::*;

            #[tokio::test]
            async fn includes_own_local_value_in_response() {
                let register: AtomicRegister<u32> = AtomicRegister::default();
                let info = register.communicate(Message::Ask).await.unwrap();

                let local = register.local.lock().unwrap();
                assert_eq!(info, vec![local.clone()])
            }
        }

        mod neighbor_urls {
            use super::*;

            #[test]
            fn appends_local_suffix() {
                let neighbor = Uri::from_static("http://test.com");
                let register = AtomicRegister::<u32>::new(vec![neighbor]);
                let urls = register.neighbor_urls();
                let url = urls.first().unwrap();
                assert_eq!(url.host().unwrap(), "test.com");
                assert_eq!(url.path(), "/register/local");
            }
        }

        mod read {
            use super::*;

            #[tokio::test]
            async fn returns_value_without_label() {
                let register: AtomicRegister<u32> = AtomicRegister::default();
                assert_eq!(0, register.read().await.unwrap())
            }
        }

        mod update {
            use super::*;

            #[test]
            fn returns_current_local_value() {
                let register: AtomicRegister<u32> = AtomicRegister::default();
                let other = LocalValue {
                    value: 123,
                    label: 123,
                };
                let local = register.update(&other);
                assert_eq!(other, local);
            }

            #[test]
            fn changes_local_value_if_other_label_is_larger() {
                let register: AtomicRegister<u32> = AtomicRegister::default();
                register.update(&LocalValue {
                    value: 123,
                    label: 123,
                });
                let local = register.local.lock().unwrap();
                assert_eq!(local.value, 123);
                assert_eq!(local.label, 123);
            }

            #[test]
            fn leaves_local_value_alone_other_label_is_smaller() {
                let register: AtomicRegister<u32> = AtomicRegister::default();
                // Update local to have non-zero label
                register.update(&LocalValue {
                    value: 123,
                    label: 123,
                });
                // Update again with smaller label
                register.update(&LocalValue { value: 1, label: 1 });
                let local = register.local.lock().unwrap();
                assert_eq!(local.value, 123);
                assert_eq!(local.label, 123);
            }
        }

        mod write {
            use super::*;

            #[tokio::test]
            async fn updates_local_to_new_value() {
                let register: AtomicRegister<u32> = AtomicRegister::default();
                register.write(123).await.unwrap();

                let local = register.local.lock().unwrap();
                assert_eq!(123, local.value);
            }

            #[tokio::test]
            async fn increases_local_label_by_one() {
                let register: AtomicRegister<u32> = AtomicRegister::default();
                register.write(123).await.unwrap();

                let local = register.local.lock().unwrap();
                assert_eq!(1, local.label);
            }
        }
    }
}
