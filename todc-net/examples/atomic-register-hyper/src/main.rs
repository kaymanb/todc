use std::net::SocketAddr;

use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::{service_fn, Service};
use hyper::{Method, Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use todc_net::register::AtomicRegister;

// The contents of the register
type Contents = String;

// The main router for our server
async fn router(
    register: AtomicRegister<Contents>,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    match (req.method(), req.uri().path()) {
        // Allow the register to be read with GET requests
        (&Method::GET, "/register") => {
            let value: String = register.read().await.unwrap();
            Ok(Response::new(Full::new(Bytes::from(value))))
        }
        // Allow the register to be written to with POST requests
        (&Method::POST, "/register") => {
            let body = req.collect().await?.to_bytes();
            let value = String::from_utf8(body.to_vec()).unwrap();
            register.write(value).await.unwrap();
            Ok(Response::new(Full::new(Bytes::new())))
        }
        // Allow the register to handle all other requests, such as
        // internal requests made to /register/local.
        _ => register.call(req).await,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create a register for this instance.
    let register: AtomicRegister<Contents> = AtomicRegister::default();

    // Create a new server with Hyper.
    let addr: SocketAddr = ([0, 0, 0, 0], 3000).into();
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let register = register.clone();
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                // Handle requests by passing them to the router
                .serve_connection(io, service_fn(move |req| router(register.clone(), req)))
                .await
            {
                println!("Error serving connection: {:?}", err)
            }
        });
    }
}
