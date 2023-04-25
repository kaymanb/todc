use std::net::{IpAddr, Ipv4Addr};

use http_body_util::BodyExt;
use hyper::server::conn::http1;
use hyper::Uri;
use serde_json::json;
use turmoil::net::TcpListener;
use turmoil::Builder;

use todc_mp::register::AtomicRegister;

mod common;
use common::{fetch_url, post_url};

async fn serve(register: AtomicRegister<u32>) -> Result<(), Box<dyn std::error::Error + 'static>> {
    let addr = (IpAddr::from(Ipv4Addr::UNSPECIFIED), 9999);
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        let register = register.clone();
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(stream, register)
                .await
            {
                println!("Internal Server Error: {:?}", err);
            }
        });
    }
}

mod register {
    use super::*;

    mod get {
        use super::*;

        mod if_one_server {
            use super::*;

            #[test]
            fn responds_with_success() {
                let mut sim = Builder::new().build();
                let register = AtomicRegister::default();
                sim.host("server1", move || serve(register.clone()));

                sim.client("client", async move {
                    let url = Uri::from_static("http://server1:9999/register");
                    let response = fetch_url(url).await.unwrap();
                    assert!(response.status().is_success());
                    Ok(())
                });

                sim.run().unwrap();
            }

            #[test]
            fn responds_with_value_as_json() {
                let mut sim = Builder::new().build();
                let register = AtomicRegister::default();
                sim.host("server1", move || serve(register.clone()));

                sim.client("client", async move {
                    let url = Uri::from_static("http://server1:9999/register");
                    let response = fetch_url(url).await.unwrap();
                    let body_bytes = response.collect().await?.to_bytes();
                    let body = std::str::from_utf8(&body_bytes)?;
                    assert_eq!(body, json!(0).to_string());
                    Ok(())
                });

                sim.run().unwrap();
            }
        }

        #[test]
        fn responds_with_success() {
            let mut sim = Builder::new().build();
            // TODO: Make serving multiple registers easier...
            let neighbors1 = vec![Uri::from_static("http://server2:9999")];
            let register1 = AtomicRegister::new(neighbors1);
            sim.host("server1", move || serve(register1.clone()));

            let register2 = AtomicRegister::default();
            sim.host("server2", move || serve(register2.clone()));

            sim.client("client", async move {
                let url = Uri::from_static("http://server1:9999/register");
                let response = fetch_url(url).await.unwrap();
                assert!(response.status().is_success());
                Ok(())
            });

            sim.run().unwrap();
        }

        #[test]
        fn responds_with_value_as_json() {
            let mut sim = Builder::new().build();
            let neighbors1 = vec![Uri::from_static("http://server2:9999")];
            let register1 = AtomicRegister::new(neighbors1);
            sim.host("server1", move || serve(register1.clone()));

            let register2 = AtomicRegister::default();
            sim.host("server2", move || serve(register2.clone()));

            sim.client("client", async move {
                let url = Uri::from_static("http://server1:9999/register");
                let response = fetch_url(url).await.unwrap();
                let body_bytes = response.collect().await?.to_bytes();
                let body = std::str::from_utf8(&body_bytes)?;
                assert_eq!(body, json!(0).to_string());
                Ok(())
            });

            sim.run().unwrap();
        }
    }
}

mod local {
    use super::*;

    mod get {
        use super::*;

        #[test]
        fn responds_with_success() {
            let mut sim = Builder::new().build();
            let register = AtomicRegister::default();
            sim.host("server1", move || serve(register.clone()));

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
            let register = AtomicRegister::default();
            sim.host("server1", move || serve(register.clone()));

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

    mod post {
        use super::*;

        #[test]
        fn responds_with_success_if_valid_request() {
            let mut sim = Builder::new().build();
            let register = AtomicRegister::default();
            sim.host("server1", move || serve(register.clone()));

            sim.client("client", async move {
                let url = Uri::from_static("http://server1:9999/register/local");
                let value = json!({"value": 0, "label": 0});
                let response = post_url(url, value.to_string()).await.unwrap();
                assert!(response.status().is_success());
                Ok(())
            });

            sim.run().unwrap();
        }

        #[test]
        fn returns_value_with_larger_label() {
            let mut sim = Builder::new().build();
            let register = AtomicRegister::default();
            sim.host("server1", move || serve(register.clone()));

            sim.client("client", async move {
                let url = Uri::from_static("http://server1:9999/register/local");
                let larger = json!({"value": 0, "label": 1});
                let response = post_url(url, larger.to_string()).await.unwrap();

                let body_bytes = response.collect().await?.to_bytes();
                let body = std::str::from_utf8(&body_bytes)?;
                assert_eq!(body, larger.to_string());
                Ok(())
            });

            sim.run().unwrap();
        }

        #[test]
        fn returns_larger_value_if_labels_are_equal() {
            let mut sim = Builder::new().build();
            let register = AtomicRegister::default();
            sim.host("server1", move || serve(register.clone()));

            sim.client("client", async move {
                let url = Uri::from_static("http://server1:9999/register/local");
                let larger = json!({"value": 1, "label": 0});
                let response = post_url(url, larger.to_string()).await.unwrap();

                let body_bytes = response.collect().await?.to_bytes();
                let body = std::str::from_utf8(&body_bytes)?;
                assert_eq!(body, larger.to_string());
                Ok(())
            });

            sim.run().unwrap();
        }

        #[test]
        fn changes_internal_value_if_request_has_larger_label() {
            let mut sim = Builder::new().build();
            let register = AtomicRegister::default();
            sim.host("server1", move || serve(register.clone()));

            sim.client("client", async move {
                let url = Uri::from_static("http://server1:9999/register/local");
                let larger = json!({"value": 0, "label": 1});
                post_url(url.clone(), larger.to_string()).await.unwrap();

                // Submit GET request to check internal value
                let response = fetch_url(url).await.unwrap();
                let body_bytes = response.collect().await?.to_bytes();
                let body = std::str::from_utf8(&body_bytes)?;
                assert_eq!(body, larger.to_string());
                Ok(())
            });

            sim.run().unwrap();
        }

        #[test]
        fn does_not_change_internal_value_if_request_has_smaller_label() {
            let mut sim = Builder::new().build();
            let register = AtomicRegister::default();
            sim.host("server1", move || serve(register.clone()));

            sim.client("client", async move {
                let url = Uri::from_static("http://server1:9999/register/local");
                // POST an initial value with larger label
                let larger = json!({"value": 0, "label": 2});
                post_url(url.clone(), larger.to_string()).await.unwrap();

                // POST a second value with smaller label
                let smaller = json!({"value": 0, "label": 1});
                post_url(url.clone(), smaller.to_string()).await.unwrap();

                // Submit GET request to check internal value
                let response = fetch_url(url).await.unwrap();
                let body_bytes = response.collect().await?.to_bytes();
                let body = std::str::from_utf8(&body_bytes)?;
                assert_eq!(body, larger.to_string());
                Ok(())
            });

            sim.run().unwrap();
        }
    }
}
