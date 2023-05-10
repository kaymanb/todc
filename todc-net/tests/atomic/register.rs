use std::net::{IpAddr, Ipv4Addr};

use bytes::Buf;
use http_body_util::BodyExt;
use hyper::server::conn::http1;
use hyper::Uri;
use rand::thread_rng; 
use rand::seq::IteratorRandom;
use serde_json::{json, Value as JSON};
use turmoil::net::TcpListener;
use turmoil::{Builder, Sim};

use todc_net::atomic::AtomicRegister;

use crate::common::{get, post};

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

fn simulate_servers<'a>(n: usize) -> Sim<'a> {
    let mut sim = Builder::new().build();

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
        sim.host(name, move || serve(register.clone()));
    }
    sim
}
 

#[test]
#[ignore] // TODO: Currently fails at weird times. Turn this into a proper test...
fn random_reads_and_writes_with_random_failures_are_linearizable() {
    const NUM_SERVERS: usize = 3;
    const FAILURE_RATE: f64 = 1.0;

    // Simulate a network where only a random majority of servers are correct.
    let mut sim = simulate_servers(NUM_SERVERS);
    sim.set_fail_rate(FAILURE_RATE); // TODO: Why do we have to set this?

    let servers: Vec<String> = (0..NUM_SERVERS)
        .map(|i| format!("{SERVER_PREFIX}-{i}"))
        .collect();
    
    let mut rng = thread_rng();
    let majority = ((NUM_SERVERS as f32 / 2.0).floor() + 1.0) as usize;
    let correct_servers = servers.clone();
    let correct_servers = correct_servers.iter().choose_multiple(&mut rng, majority);
    
    println!("{correct_servers:?}");

    // TODO: This needs to change...
    for (correct, server) in correct_servers.into_iter().zip(servers.into_iter()) {
        if *correct == server { continue };
        let a = correct.clone();
        let b = server.clone();
        sim.set_link_fail_rate(a, b, 0.0);

    }

    sim.client("client", async move {
        let url = Uri::from_static("http://server-0:9999/register");
        let response = get(url).await.unwrap();
        assert!(response.status().is_success());
        Ok(())
    });
    sim.set_link_fail_rate("client", "server-0", 0.0);

    sim.run().unwrap();
}

#[test]
/// Test that if two writes happen concurrently, and one is delayed
/// long enough for the other to be succefully applied, then the former
/// is not also applied to the register when it completes.
///
/// The delayed write cannot be applied because an individual server cannot
/// tell if the write was _actually_ delayed, or if it is just receiving very
/// old messages. If it is the latter, and the write has already been applied,
/// then applying it again would mean that future reads may return an
/// incorrect value.
fn delayed_write_is_not_applied() {
    let mut sim = simulate_servers(3);
    sim.client("client", async move {
        let first = json!(10);
        let second = json!(20);

        turmoil::hold("server-0", "server-1");
        turmoil::hold("server-0", "server-2");

        // A first write is performed on server-0, which will initially be delayed
        // due to the network latency, but eventually complete.
        let handle = tokio::task::spawn(async move {
            let url = Uri::from_static("http://server-0:9999/register");
            let response = post(url, first).await.unwrap();
            assert!(response.status().is_success());
        });
        tokio::task::yield_now().await;

        // Assert that the first write has not yet been applied.
        let url = Uri::from_static("http://server-1:9999/register");
        let response = get(url).await.unwrap();
        let body = response.collect().await?.aggregate();
        let value: JSON = serde_json::from_reader(body.reader())?;
        assert_eq!(value, json!(0));

        // A second write is performed, and will be applied immiediately,
        // because server-1 is only partially-affected by the hold.
        let url = Uri::from_static("http://server-1:9999/register");
        post(url, second.clone()).await.unwrap();

        // Assert that second write was applied.
        let url = Uri::from_static("http://server-1:9999/register");
        let response = get(url).await.unwrap();
        let body = response.collect().await?.aggregate();
        let value: JSON = serde_json::from_reader(body.reader())?;
        assert_eq!(value, second);

        // Release messages, and wait for the first write to complete.
        turmoil::release("server-0", "server-1");
        turmoil::release("server-0", "server-2");
        handle.await?;

        // Assert that the value from the first write was not applied.
        let url = Uri::from_static("http://server-1:9999/register");
        let response = get(url).await.unwrap();
        let body = response.collect().await?.aggregate();
        let value: JSON = serde_json::from_reader(body.reader())?;
        assert_eq!(value, second);
        Ok(())
    });
    sim.run().unwrap();
}

mod get {
    use super::*;

    #[test]
    fn responds_with_success() {
        let mut sim = simulate_servers(2);

        sim.client("client", async move {
            let url = Uri::from_static("http://server-0:9999/register");
            let response = get(url).await.unwrap();
            assert!(response.status().is_success());
            Ok(())
        });
        sim.run().unwrap();
    }

    #[test]
    fn returns_value_as_json() {
        let mut sim = simulate_servers(2);
        sim.client("client", async move {
            let url = Uri::from_static("http://server-0:9999/register");
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
        let mut sim = simulate_servers(2);
        sim.client("client", async move {
            // Set local value of server2
            let url2 = Uri::from_static("http://server-1:9999/register/local");
            let value = 123;
            let larger = json!({"value": value, "label": 1});
            post(url2.clone(), larger).await.unwrap();

            // Perform read operation on server1
            let url = Uri::from_static("http://server-0:9999/register");
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
        let mut sim = simulate_servers(2);
        sim.client("client", async move {
            // Set local value of server1
            let local_url = Uri::from_static("http://server-0:9999/register/local");
            let value = 123;
            let larger = json!({"value": value, "label": 1});
            post(local_url, larger.clone()).await.unwrap();

            // Perform read operation on server1
            let url = Uri::from_static("http://server-0:9999/register");
            get(url).await.unwrap();

            // Check the local value of server2
            let url2 = Uri::from_static("http://server-1:9999/register/local");
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
        let mut sim = simulate_servers(3);
        sim.client("client", async move {
            turmoil::hold("server-0", "server-1");

            let url = Uri::from_static("http://server-0:9999/register");
            let response = get(url).await.unwrap();
            let body = response.collect().await?.aggregate();
            let body: JSON = serde_json::from_reader(body.reader())?;
            assert_eq!(body, json!(0));
            Ok(())
        });
        sim.run().unwrap();
    }
}

mod post {
    use super::*;

    #[test]
    fn responds_with_success() {
        let mut sim = simulate_servers(2);
        sim.client("client", async move {
            let url = Uri::from_static("http://server-0:9999/register");
            let response = post(url, json!(123)).await.unwrap();
            assert!(response.status().is_success());
            Ok(())
        });
        sim.run().unwrap();
    }

    #[test]
    fn returns_empty_body() {
        let mut sim = simulate_servers(2);
        sim.client("client", async move {
            let url = Uri::from_static("http://server-0:9999/register");
            let response = post(url, json!(123)).await.unwrap();
            let body = response.collect().await?.aggregate();
            let body: JSON = serde_json::from_reader(body.reader())?;
            assert_eq!(body, JSON::Null);
            Ok(())
        });
        sim.run().unwrap();
    }

    #[test]
    fn announces_value_to_neighbors() {
        let mut sim = simulate_servers(2);
        sim.client("client", async move {
            // Write value to register
            let url = Uri::from_static("http://server-0:9999/register");
            post(url, json!(123)).await.unwrap();

            // Check that value was adopted by neighbor
            let url = Uri::from_static("http://server-1:9999/register/local");
            let response = get(url).await.unwrap();
            let body = response.collect().await?.aggregate();
            let body: JSON = serde_json::from_reader(body.reader())?;
            assert_eq!(body, json!({ "label": 1, "value": 123 }));
            Ok(())
        });
        sim.run().unwrap();
    }

    #[test]
    fn responds_even_if_half_of_neighbors_are_offline() {
        let mut sim = simulate_servers(3);
        sim.client("client", async move {
            turmoil::hold("server-0", "server-1");

            let url = Uri::from_static("http://server-0:9999/register");
            let response = post(url, json!(123)).await.unwrap();
            assert!(response.status().is_success());
            Ok(())
        });
        sim.run().unwrap();
    }
}

mod local {
    use super::*;

    mod get {
        use super::*;

        #[test]
        fn responds_with_success() {
            let mut sim = simulate_servers(1);
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
            let mut sim = simulate_servers(1);
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
            let mut sim = simulate_servers(1);
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
            let mut sim = simulate_servers(1);
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
            let mut sim = simulate_servers(1);
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
            let mut sim = simulate_servers(1);
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
            let mut sim = simulate_servers(1);
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
