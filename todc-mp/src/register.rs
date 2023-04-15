//! An atomic register based on the implementation by Attiya, Bar-Noy, and
//! Dolev [[ABD95]](https://dl.acm.org/doi/pdf/10.1145/200836.200869).
use std::future::Future;
use std::pin::Pin;

use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::service::Service;
use hyper::{Error, Method, Request, Response, Uri};
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Clone, Debug, Deserialize, Serialize, Default, PartialEq, Eq, PartialOrd, Ord)]
struct LocalValue<T: Default + Serialize> {
    label: u32,
    value: T,
}

#[derive(Clone, Debug)]
struct AtomicRegister<T: Default + Serialize> {
    neighbors: Vec<Uri>,
    value: LocalValue<T>,
}

impl<T: Default + Serialize> AtomicRegister<T> {
    fn new(neighbors: Vec<Uri>) -> Self {
        Self {
            neighbors,
            value: LocalValue::<T>::default(),
        }
    }

    fn route(&mut self, method: &Method, path: &str) -> Result<Response<Full<Bytes>>, Error> {
        // TODO: Refactor how responses are sent.
        fn mk_response(s: &str) -> Result<Response<Full<Bytes>>, Error> {
            Ok(Response::builder()
                .body(Full::new(Bytes::from(s.to_string())))
                .unwrap())
        }

        match (method, path) {
            (&Method::GET, "/register") => mk_response("GET /register"), // Read
            (&Method::POST, "/register") => mk_response("POST /register"), // Write
            // Internal Only Routes
            (&Method::GET, "/register/local") => {
                let value_str = serde_json::to_string(&self.value).unwrap_or("Oops!".to_string());
                mk_response(&value_str)
            }
            (&Method::POST, "/register/local") => mk_response("POST /register/local"), // Update and return internal value
            // TODO: Probably shouldn't error on other routes..
            _ => mk_response("Oops! Not Found".into()),
        }
    }
}

impl<T: Default + Serialize> Service<Request<Incoming>> for AtomicRegister<T> {
    type Response = Response<Full<Bytes>>;
    type Error = Error;
    // TODO: Define a concrete Future type to avoid Box here.
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    // TODO: This is only required because you cannot create an instance of `Incoming`
    // while unit testing
    fn call(&mut self, req: Request<Incoming>) -> Self::Future {
        let res = self.route(&req.method(), req.uri().path());
        Box::pin(async { res })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    type Value = u32;

    fn register() -> AtomicRegister<Value> {
        AtomicRegister::<u32>::new(vec![Uri::from_static("http://test.com")])
    }

    mod local {
        use super::*;
        const URI: &str = "http://test.com/register/local";

        mod get {
            use super::*;
            const METHOD: &Method = &Method::GET;

            #[test]
            fn responds_with_success() {
                let mut register = register();
                let uri = Uri::from_static(URI);
                let res = register.route(METHOD, uri.path()).unwrap();
                assert!(res.status().is_success())
            }

            #[tokio::test]
            async fn responds_with_json_body() {
                let mut register = register();
                let uri = Uri::from_static(URI);
                let res = register.route(METHOD, uri.path()).unwrap();
                let body = res.collect().await.unwrap().to_bytes();

                let local_value: LocalValue<Value> = serde_json::from_slice(&body).unwrap();
                assert_eq!(local_value.value, 0);
                assert_eq!(local_value.label, 0);
            }
        }
    }
}
