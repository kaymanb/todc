# todc-net

[![crates.io](https://img.shields.io/crates/v/todc-net)](https://crates.io/crates/todc-net/)
[![docs.rs](https://img.shields.io/docsrs/todc-net)](https://docs.rs/todc-net/0.1.0/todc_net/)

Algorithms for message-passing (HTTP) distributed systems.

## Examples

In the following example, we create a single instance of the register that
will expose read and write operations as HTTP requests to `/register`. For
this example, our register will hold a type `String`.

We can use [`hyper`](https://docs.rs/hyper/latest/hyper/) to run a local instance of the register as follows:

```rust
use std::net::SocketAddr;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request, Response};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::{Service, service_fn};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use todc_net::register::AtomicRegister;

// The contents of the register
type Contents = String;

// The main router for our server
async fn router(
    register: AtomicRegister<Contents>,
    req: Request<Incoming>
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    match (req.method(), req.uri().path()) {
        // Allow the register to be read with GET requests
        (&Method::GET, "/register") => {
            let value: String = register.read().await.unwrap();
            Ok(Response::new(Full::new(Bytes::from(value))))
        },
        // Allow the register to be written to with POST requests
        (&Method::POST, "/register") => {
            let body = req.collect().await?.to_bytes();
            let value = String::from_utf8(body.to_vec()).unwrap();
            register.write(value).await.unwrap();
            Ok(Response::new(Full::new(Bytes::new())))
        },
        // Allow the register to handle all other requests, such as
        // internal requests made to /register/local.
        _ => register.call(req).await
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
```

### Interacting with a Register

Although this register isn't fault-tolerant yet, we can still try it out. See
the runnable example at [`todc-net/examples/atomic-register-hyper`](https://github.com/kaymanb/todc/tree/main/todc-net/examples/atomic-register-hyper).

## Adding Fault Tolerance with Multiple Instances

To make our register fault tolerant, we need to add more instances. Suppose that
we want `3` instances, so that even if one instance fails the register will
continue to be available.

If we have configured our infrastructure so that for each `i` in `[1, 2, 3]` the
server hosting instance `i` will be available at `https://my-register-{i}.com`, and
we have exposed `i` as an environmental variable `INSTANCE_ORDINAL`, then we
can instantiate the `AtomicRegister` as follows:

```rust
use std::env;

// Replacement for `let register = AtomicRegister::default();`
let instance_ordinal: u32 = env::var("INSTANCE_ORDINAL").unwrap().parse().unwrap();
let neighbor_urls: Vec<Uri> = (1..4)
    .filter(|&i| i != instance_ordinal)
    .map(|i| format!("https://my-register-{i}.com").parse().unwrap())
    .collect();
let register: AtomicRegister<Contents> = AtomicRegister::new(neighbor_urls);
```

### Interacting with a Fault Tolerant Register

To interact with a fault-tolerant register backed by multiple instances, see
the runnable example at
[`todc-net/examples/atomic-register-docker-minikube`](https://github.com/kaymanb/todc/tree/main/todc-net/examples/atomic-register-docker-minikube).

## Development

Some tests make use of [turmoil](https://github.com/tokio-rs/turmoil) to
simulate latency and failures within a network. To run tests that require this
feature, do:
```
cargo test --features turmoil --test MODULE
```
