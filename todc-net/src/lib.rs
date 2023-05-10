use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, Uri};
use serde_json::{json, Value as JSON};

use crate::net::TcpStream;

pub mod atomic;
pub mod net;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type ResponseResult = Result<Response<Incoming>, GenericError>;

pub(crate) async fn get(url: Uri) -> ResponseResult {
    make_request(url, Method::GET, json!(null)).await
}

pub(crate) async fn post(url: Uri, body: JSON) -> ResponseResult {
    make_request(url, Method::POST, body).await
}

async fn make_request(url: Uri, method: Method, body: JSON) -> ResponseResult {
    let authority = url.authority().ok_or("Invalid URL")?.as_str();
    let stream = TcpStream::connect(authority).await?;
    let (mut sender, conn) = hyper::client::conn::http1::handshake(stream).await?;

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

fn full(value: JSON) -> BoxBody<Bytes, hyper::Error> {
    Full::<Bytes>::new(Bytes::from(value.to_string()))
        .map_err(|never| match never {})
        .boxed()
}
