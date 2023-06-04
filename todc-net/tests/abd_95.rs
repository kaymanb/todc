use std::net::{IpAddr, Ipv4Addr};

use bytes::Buf;
use http_body_util::BodyExt;
use hyper::server::conn::http1;
use hyper::Uri;
use serde_json::{json, Value as JSON};
use turmoil::net::TcpListener;
use turmoil::{Builder, Sim};

use todc_net::abd_95::AtomicRegister;

mod common;
use common::{get, post};

mod abd_95 {
    mod linearizability;
}

const SERVER_PREFIX: &str = "server";
const PORT: u32 = 9999;

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

fn simulate_servers<'a>(n: usize) -> (Sim<'a>, Vec<AtomicRegister<u32>>) {
    let mut sim = Builder::new().build();

    let mut registers = Vec::new();

    let neighbors: Vec<Uri> = (0..n)
        .map(|i| {
            format!("http://{SERVER_PREFIX}-{i}:{PORT}")
                .parse()
                .unwrap()
        })
        .collect();

    for i in 0..n {
        let mut neighbors = neighbors.clone();
        neighbors.remove(i);
        let register: AtomicRegister<u32> = AtomicRegister::new(neighbors);
        let name = format!("{SERVER_PREFIX}-{i}");
        let register_clone = register.clone();
        sim.host(name, move || serve(register_clone.clone()));
        registers.push(register);
    }
    (sim, registers)
}

mod read {
    use super::*;

    #[test]
    fn returns_current_value() {
        let (mut sim, replicas) = simulate_servers(2);
        sim.client("client", async move {
            let value = replicas[0].read().await.unwrap();
            assert_eq!(value, 0);
            Ok(())
        });
        sim.run().unwrap();
    }

    #[test]
    fn returns_value_from_write_to_other_replica() {
        const VALUE: u32 = 123;
        let (mut sim, replicas) = simulate_servers(2);
        sim.client("client", async move {
            replicas[1].write(VALUE).await.unwrap();
            let value = replicas[0].read().await.unwrap();
            assert_eq!(value, VALUE);
            Ok(())
        });
        sim.run().unwrap();
    }

    #[test]
    fn returns_even_if_half_of_neighbors_are_unreachable() {
        let (mut sim, replicas) = simulate_servers(3);
        sim.client("client", async move {
            turmoil::hold("client", "server-1");
            let value = replicas[0].read().await.unwrap();
            assert_eq!(value, 0);
            Ok(())
        });
        sim.run().unwrap();
    }

    #[test]
    fn hangs_if_more_than_half_of_neighbors_are_unreachable() {
        let (mut sim, replicas) = simulate_servers(3);
        sim.client("client", async move {
            turmoil::hold("client", "server-1");
            turmoil::hold("client", "server-2");

            replicas[0].read().await.unwrap();

            Ok(())
        });

        assert!(sim
            .run()
            .unwrap_err()
            .to_string()
            .contains("Ran for 10s without completing"))
    }
}

mod write {
    use super::*;

    #[test]
    fn returns_nothing() {
        let (mut sim, replicas) = simulate_servers(2);
        sim.client("client", async move {
            let value = replicas[0].write(123).await.unwrap();
            assert_eq!(value, ());
            Ok(())
        });
        sim.run().unwrap();
    }

    #[test]
    fn sets_value_of_requested_replica() {
        let (mut sim, replicas) = simulate_servers(2);
        sim.client("client", async move {
            replicas[0].write(123).await.unwrap();
            let value = replicas[0].read().await.unwrap();
            assert_eq!(value, 123);
            Ok(())
        });
        sim.run().unwrap();
    }

    #[test]
    fn sets_value_of_all_other_replicas() {
        const NUM_REPLICAS: usize = 3;
        const VALUE: u32 = 123;
        let (mut sim, replicas) = simulate_servers(NUM_REPLICAS);
        sim.client("client", async move {
            replicas[0].write(VALUE).await.unwrap();
            for i in (0..NUM_REPLICAS).rev() {
                let value = replicas[i].read().await.unwrap();
                assert_eq!(value, VALUE);
            }
            Ok(())
        });
        sim.run().unwrap();
    }

    #[test]
    fn returns_even_if_half_of_neighbors_are_unreachable() {
        let (mut sim, replicas) = simulate_servers(3);
        sim.client("client", async move {
            turmoil::hold("client", "server-1");
            replicas[0].write(123).await.unwrap();
            let value = replicas[0].read().await.unwrap();
            assert_eq!(value, 123);
            Ok(())
        });
        sim.run().unwrap();
    }

    #[test]
    fn hangs_if_more_than_half_of_neighbors_are_offline() {
        let (mut sim, replicas) = simulate_servers(3);
        sim.client("client", async move {
            turmoil::hold("client", "server-1");
            turmoil::hold("client", "server-2");
            replicas[0].write(123).await.unwrap();
            Ok(())
        });

        assert!(sim
            .run()
            .unwrap_err()
            .to_string()
            .contains("Ran for 10s without completing"))
    }
}

mod local {
    use super::*;

    mod get {
        use super::*;

        #[test]
        fn responds_with_success() {
            let (mut sim, _) = simulate_servers(1);
            sim.client("client", async move {
                let url = Uri::from_static("http://server-0:9999/register/local");
                let response = get(url).await.unwrap();
                assert!(response.status().is_success());
                Ok(())
            });
            sim.run().unwrap();
        }

        #[test]
        fn responds_with_local_value_as_json() {
            let (mut sim, _) = simulate_servers(1);
            sim.client("client", async move {
                let url = Uri::from_static("http://server-0:9999/register/local");
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
            let (mut sim, _) = simulate_servers(1);
            sim.client("client", async move {
                let url = Uri::from_static("http://server-0:9999/register/local");
                let value = json!({"value": 0, "label": 0});
                let response = post(url, value).await.unwrap();
                assert!(response.status().is_success());
                Ok(())
            });
            sim.run().unwrap();
        }

        #[test]
        fn returns_value_with_larger_label() {
            let (mut sim, _) = simulate_servers(1);
            sim.client("client", async move {
                let url = Uri::from_static("http://server-0:9999/register/local");
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
            let (mut sim, _) = simulate_servers(1);
            sim.client("client", async move {
                let url = Uri::from_static("http://server-0:9999/register/local");
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
            let (mut sim, _) = simulate_servers(1);
            sim.client("client", async move {
                let url = Uri::from_static("http://server-0:9999/register/local");
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
            let (mut sim, _) = simulate_servers(1);
            sim.client("client", async move {
                let url = Uri::from_static("http://server-0:9999/register/local");
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
