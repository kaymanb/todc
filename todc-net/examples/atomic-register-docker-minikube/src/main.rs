use std::env;
use std::fmt::Debug;
use std::net::SocketAddr;

use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::{Service, service_fn};
use hyper::{Method, Request, Response, Uri};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;
use tokio::net::TcpListener;
use tracing_subscriber;

// TODO: mk_response should not be public...
use todc_net::mk_response;
use todc_net::atomic::register::abd_95::AtomicRegister;

async fn router<T: Clone + Debug + Default + DeserializeOwned + Ord + Send + Serialize + 'static>(
    mut register: AtomicRegister<T>, 
    req: Request<Incoming>
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => mk_response(json!("Try submitting requests to /register!")),
        _ => register.call(req).await
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
            format!("http://{app_name}-{i}.default.svc.cluster.local")
                .parse()
                .unwrap()
        })
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let addr: SocketAddr = ([0, 0, 0, 0], 3000).into();

    let neighbors = find_neighbors();
    let register: AtomicRegister<String> = AtomicRegister::new(neighbors);

    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);
    loop {
        let (stream, _) = listener.accept().await?;
        
        let register = register.clone();
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(stream, service_fn(move |req| {
                    let register = register.clone();
                    router(register, req)
                }))
                .await
            {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }
}
