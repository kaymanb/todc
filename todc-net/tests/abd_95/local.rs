use bytes::Buf;
use http_body_util::BodyExt;
use hyper::Uri;
use serde_json::{json, Value as JSON};

use crate::abd_95::common::{get, post};
use crate::simulate_servers;

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
