use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::body::Incoming;
use hyper::{Request, Response, Uri};
use serde_json::Value as JSON;
use turmoil::net::TcpStream;

// A simple type alias so as to DRY.
type FetchResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub async fn get(url: Uri) -> FetchResult<Response<Incoming>> {
    let host = url.host().expect("uri has no host");
    let port = url.port_u16().unwrap_or(80);
    let addr = format!("{host}:{port}");
    let stream = TcpStream::connect(addr).await?;

    let (mut sender, conn) = hyper::client::conn::http1::handshake(stream).await?;
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {err}");
        }
    });

    let authority = url.authority().unwrap().clone();

    let req = Request::builder()
        .uri(url)
        .header(hyper::header::HOST, authority.as_str())
        .body(empty())?;

    let res = sender.send_request(req).await?;
    Ok(res)
}

pub async fn post(url: Uri, body: JSON) -> FetchResult<Response<Incoming>> {
    let host = url.host().expect("uri has no host");
    let port = url.port_u16().unwrap_or(80);
    let addr = format!("{host}:{port}");
    let stream = TcpStream::connect(addr).await?;

    let (mut sender, conn) = hyper::client::conn::http1::handshake(stream).await?;
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {err}");
        }
    });

    let authority = url.authority().unwrap().clone();

    let req = Request::builder()
        .uri(url)
        .header(hyper::header::HOST, authority.as_str())
        .method("POST")
        .body(full(body))?;

    let res = sender.send_request(req).await?;
    Ok(res)
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

fn full(value: JSON) -> BoxBody<Bytes, hyper::Error> {
    Full::<Bytes>::new(Bytes::from(value.to_string()))
        .map_err(|never| match never {})
        .boxed()
}
