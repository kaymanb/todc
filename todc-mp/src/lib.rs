use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::body::Bytes;
use hyper::{Method, Request, Response, StatusCode, Uri};

pub mod net;
pub mod register;

use crate::net::TcpStream;

// A simple type alias so as to DRY.
type FetchResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub async fn echo(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => Ok(Response::new(full("Echo!"))),
        (&Method::GET, "/register") => {
            let result = fetch_url("http://server2:9999".parse::<Uri>().unwrap())
                .await
                .unwrap_or(Bytes::from("Oops!"));
            // let result_str = std::str::from_utf8(&result).unwrap_or("Oops!");
            Ok(Response::new(full(result)))
        }
        _ => {
            let mut not_found = Response::new(empty());
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}
fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

pub async fn fetch_url(url: Uri) -> FetchResult<Bytes> {
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
    let body = res.collect().await?.to_bytes();
    Ok(body)
}
