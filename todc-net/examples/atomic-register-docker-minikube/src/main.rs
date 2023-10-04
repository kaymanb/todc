use std::env;
use std::net::SocketAddr;

use bytes::{Buf, Bytes};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::http::StatusCode;
use hyper::server::conn::http1;
use hyper::service::{service_fn, Service};
use hyper::{Method, Request, Response, Uri};
use hyper_util::rt::TokioIo;
use serde_json::{json, Value as JSON};
use tokio::net::TcpListener;

use todc_net::abd_95::AtomicRegister;

fn mk_response(body: JSON) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

/// Routes requests to the appropriate register operations.
async fn router(
    register: AtomicRegister<String>,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => Ok(mk_response(json!("Try submitting requests to /register!"))),
        (&Method::GET, "/register") => {
            let value: String = register.read().await.unwrap();
            let body = serde_json::to_value(value).unwrap();
            Ok(mk_response(body))
        }
        (&Method::POST, "/register") => {
            let body = req.collect().await?.aggregate();
            let value: JSON = serde_json::from_reader(body.reader())?;
            register.write(value.to_string()).await.unwrap();
            Ok(mk_response(json!(null)))
        }
        _ => register.call(req).await,
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
            format!("http://{app_name}-{i}.default.svc.cluster.local:3000")
                .parse()
                .unwrap()
        })
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr: SocketAddr = ([0, 0, 0, 0], 3000).into();

    let neighbors = find_neighbors();
    let register: AtomicRegister<String> = AtomicRegister::new(neighbors);

    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let register = register.clone();
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(move |req| router(register.clone(), req)))
                .await
            {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }
}
