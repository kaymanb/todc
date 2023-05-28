use std::env;
use std::net::SocketAddr;

use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode, Uri};
use todc_net::atomic::register::abd_95::AtomicRegister;
use tokio::net::TcpListener;

async fn router(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => Ok(Response::new(full("Try submitting requests to /register!"))),

        _ => {
            let mut not_found = Response::new(full(""));
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

/// Returns a vector containing the URL of all neighboring
/// AtomicRegister instances in the local cluster.
fn find_neighbors() -> Vec<Uri> {
    let pod_name =
        env::var("POD_NAME").expect("environmental variable 'POD_NAME' should be set by K8s");

    let (app_name, ordinal_str) = pod_name
        .rsplit_once('-')
        .expect("pod name should be of the format {APP_NAME}-{ORDINAL}");
    println!("App Name: {app_name:?}");

    let ordinal: u32 = ordinal_str.parse().expect("Ordinal should be a valid u32");
    println!("Ordinal: {ordinal:?}");

    let num_replicas: u32 = env::var("NUM_REPLICAS")
        .expect("environmental variable 'NUM_RECORDS' should be set by K8s")
        .parse()
        .expect("environmental variable 'NUM_RECORDS' should be valid u32");
    println!("Number of Replicas: {num_replicas:?}");

    (0..num_replicas)
        .filter(|i| i != &ordinal)
        .map(|i| {
            format!("http://{app_name}-{i}.svc.cluster.local")
                .parse()
                .unwrap()
        })
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr: SocketAddr = ([0, 0, 0, 0], 3000).into();

    let neighbors = find_neighbors();
    let _register: AtomicRegister<String> = AtomicRegister::new(neighbors);

    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);
    loop {
        let (stream, _) = listener.accept().await?;
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(stream, service_fn(router))
                .await
            {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}
