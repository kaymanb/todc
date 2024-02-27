//! Algorithms for message-passing (HTTP) distributed systems.
use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::http::StatusCode;
use hyper::{Method, Request, Response, Uri};
use serde_json::{json, Value as JSON};

use crate::net::TcpStream;

pub(crate) mod net;
pub mod register;

// NOTE: This module adds a local copy of some helper types that for integrating
// tokio with Hyper 1.0. Hopefully, once Hyper 1.0 is released, there will be
// a more standard way to integrate and this module can be deleted.
// See: https://github.com/hyperium/hyper/issues/3110
mod hyper_util_tokio_io;
pub use hyper_util_tokio_io::TokioIo;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type ResponseResult = Result<Response<Incoming>, GenericError>;

/// Submits a GET request to the URL.
pub(crate) async fn get(url: Uri) -> ResponseResult {
    make_request(url, Method::GET, json!(null)).await
}

/// Submits a POST request, along with a JSON body, to the URL.
pub(crate) async fn post(url: Uri, body: JSON) -> ResponseResult {
    make_request(url, Method::POST, body).await
}

/// Makes a request to the URL, including a JSON body.
async fn make_request(url: Uri, method: Method, body: JSON) -> ResponseResult {
    let authority = url.authority().ok_or("Invalid URL")?.as_str();
    let stream = TcpStream::connect(authority).await?;

    // Use adapter to access something implementing tokio::io as if they
    // implement hyper::rt.
    // See: https://github.com/hyperium/hyper/issues/3110
    let io = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {err}");
        }
    });

    let req = Request::builder()
        .header(hyper::header::HOST, authority)
        .uri(url)
        .method(method)
        .body(full(body))?;

    Ok(sender.send_request(req).await?)
}

/// Creates a response containing a JSON value.
pub(crate) fn mk_response(
    status: StatusCode,
    body: JSON,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::builder()
        .status(status)
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap())
}

/// Returns a JSON body.
fn full(value: JSON) -> BoxBody<Bytes, hyper::Error> {
    Full::<Bytes>::new(Bytes::from(value.to_string()))
        .map_err(|never| match never {})
        .boxed()
}
