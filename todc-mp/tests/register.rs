use std::net::{IpAddr, Ipv4Addr};

use bytes::Buf;
use http_body_util::BodyExt;
use hyper::server::conn::http1;
use hyper::Uri;
use serde_json::{json, Value as JSON};
use turmoil::net::TcpListener;
use turmoil::Builder;


use todc_mp::register::AtomicRegister;

mod common;
use common::{get, post};

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
                    let response = get(url).await.unwrap();
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
                    let response = get(url).await.unwrap();
                    let body = response.collect().await?.aggregate();
                    let body: JSON = serde_json::from_reader(body.reader())?;
                    assert_eq!(body, json!(0));
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
                let response = get(url).await.unwrap();
                assert!(response.status().is_success());
                Ok(())
            });

            sim.run().unwrap();
        }

        #[test]
        fn returns_value_as_json() {
            let mut sim = Builder::new().build();
            let neighbors1 = vec![Uri::from_static("http://server2:9999")];
            let register1 = AtomicRegister::new(neighbors1);
            sim.host("server1", move || serve(register1.clone()));

            let register2 = AtomicRegister::default();
            sim.host("server2", move || serve(register2.clone()));

            sim.client("client", async move {
                let url = Uri::from_static("http://server1:9999/register");
                let response = get(url).await.unwrap();
                let body = response.collect().await?.aggregate();
                let body: JSON = serde_json::from_reader(body.reader())?;
                assert_eq!(body, json!(0));
                Ok(())
            });

            sim.run().unwrap();
        }

        #[test]
        fn returns_value_from_other_server_with_larger_label() {
            let mut sim = Builder::new().build();
            let neighbors1 = vec![Uri::from_static("http://server2:9999")];
            let register1 = AtomicRegister::new(neighbors1);
            sim.host("server1", move || serve(register1.clone()));

            let register2 = AtomicRegister::default();
            sim.host("server2", move || serve(register2.clone()));

            sim.client("client", async move {
                // Set local value of server2
                let url2 = Uri::from_static("http://server2:9999/register/local");
                let value = 123;
                let larger = json!({"value": value, "label": 1});
                post(url2.clone(), larger).await.unwrap();

                // Perform read operation on server1
                let url = Uri::from_static("http://server1:9999/register");
                let response = get(url).await.unwrap();
                let body = response.collect().await?.aggregate();
                let body: JSON = serde_json::from_reader(body.reader())?;
                assert_eq!(body, json!(value));
                Ok(())
            });

            sim.run().unwrap();
        }

        #[test]
        fn announces_returned_value_to_other_servers() {
            let mut sim = Builder::new().build();
            let neighbors1 = vec![
                Uri::from_static("http://server2:9999"),
            ];
            let register1 = AtomicRegister::new(neighbors1);
            sim.host("server1", move || serve(register1.clone()));

            let register2 = AtomicRegister::default();
            sim.host("server2", move || serve(register2.clone()));

            sim.client("client", async move {
                // Set local value of server1
                let local_url = Uri::from_static("http://server1:9999/register/local");
                let value = 123;
                let larger = json!({"value": value, "label": 1});
                post(local_url, larger.clone()).await.unwrap();

                // Perform read operation on server1
                let url = Uri::from_static("http://server1:9999/register");
                get(url).await.unwrap();
                
                // Check the local value of server2
                let url2 = Uri::from_static("http://server2:9999/register/local");
                let response = get(url2).await.unwrap();
                let body = response.collect().await?.aggregate();
                let local2: JSON = serde_json::from_reader(body.reader())?;
                assert!(local2 == larger);
                Ok(())
            });

            sim.run().unwrap();
        }

        #[test]
        fn responds_even_if_half_of_neighbors_are_offline() {
            let mut sim = Builder::new().build();
            let neighbors1 = vec![
                Uri::from_static("http://server2:9999"),
                Uri::from_static("http://server3:9999")
            ];
            let register1 = AtomicRegister::new(neighbors1);
            sim.host("server1", move || serve(register1.clone()));

            let register2 = AtomicRegister::default();
            sim.host("server2", move || serve(register2.clone()));

            let register3 = AtomicRegister::default();
            sim.host("server3", move || serve(register3.clone()));

            sim.client("client", async move {
                turmoil::partition("server1", "server2");

                let url = Uri::from_static("http://server1:9999/register");
                let response = get(url).await.unwrap();
                let body = response.collect().await?.aggregate();
                let body: JSON = serde_json::from_reader(body.reader())?;
                assert_eq!(body, json!(0));
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
                let response = get(url).await.unwrap();
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
                let response = get(url).await.unwrap();
                let body = response.collect().await?.aggregate();
                let body: JSON = serde_json::from_reader(body.reader())?;
                assert_eq!(body, json!({"value": 0, "label": 0}));
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
                let response = post(url, value).await.unwrap();
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
                let response = post(url, larger.clone()).await.unwrap();

                let body = response.collect().await?.aggregate();
                let body: JSON = serde_json::from_reader(body.reader())?;
                assert_eq!(body, larger);
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
                let response = post(url, larger.clone()).await.unwrap();

                let body = response.collect().await?.aggregate();
                let body: JSON = serde_json::from_reader(body.reader())?;
                assert_eq!(body, larger);
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
                post(url.clone(), larger.clone()).await.unwrap();

                // Submit GET request to check internal value
                let response = get(url).await.unwrap();
                let body = response.collect().await?.aggregate();
                let body: JSON = serde_json::from_reader(body.reader())?;
                assert_eq!(body, larger);
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
                post(url.clone(), larger.clone()).await.unwrap();

                // POST a second value with smaller label
                let smaller = json!({"value": 0, "label": 1});
                post(url.clone(), smaller).await.unwrap();

                // Submit GET request to check internal value
                let response = get(url).await.unwrap();
                let body = response.collect().await?.aggregate();
                let body: JSON = serde_json::from_reader(body.reader())?;
                assert_eq!(body, larger);
                Ok(())
            });

            sim.run().unwrap();
        }
    }
}
