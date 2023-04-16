use std::net::{IpAddr, Ipv4Addr};

use http_body_util::BodyExt;
use hyper::server::conn::http1;
use hyper::Uri;
use serde_json::json;
use turmoil::net::TcpListener;
use turmoil::Builder;

use todc_mp::register::AtomicRegister;

mod common;
use common::fetch_url;

async fn serve_register() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let addr = (IpAddr::from(Ipv4Addr::UNSPECIFIED), 9999);
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(stream, AtomicRegister::<u32>::new())
                .await
            {
                println!("error serving connection: {:?}", err);
            }
        });
    }
}
mod local {
    use super::*;

    mod get {
        use super::*;

        #[test]
        fn responds_with_success() {
            let mut sim = Builder::new().build();

            sim.host("server1", move || serve_register());

            sim.client("client", async move {
                let url = Uri::from_static("http://server1:9999/register/local");
                let response = fetch_url(url).await.unwrap();
                assert!(response.status().is_success());
                Ok(())
            });

            sim.run().unwrap();
        }

        #[test]
        fn responds_with_local_value_as_json() {
            let mut sim = Builder::new().build();

            sim.host("server1", move || serve_register());

            sim.client("client", async move {
                let url = Uri::from_static("http://server1:9999/register/local");
                let response = fetch_url(url).await.unwrap();
                let body_bytes = response.collect().await?.to_bytes();
                let body = std::str::from_utf8(&body_bytes)?;
                assert_eq!(body, json!({"value": 0, "label": 0}).to_string());
                Ok(())
            });

            sim.run().unwrap();
        }
    }
}
