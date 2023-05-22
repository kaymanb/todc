use std::error::Error;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bytes::Buf;
use http_body_util::BodyExt;
use hyper::Uri;
use rand::distributions::Standard;
use rand::prelude::Distribution;
use rand::rngs::ThreadRng;
use rand::seq::IteratorRandom;
use rand::{thread_rng, Rng};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::json;

use todc_utils::linearizability::history::{Action, History};
use todc_utils::linearizability::WLGChecker;
use todc_utils::specifications::register::{RegisterOperation, RegisterSpecification};

use crate::common::{get, post};
use crate::register::{simulate_servers, SERVER_PREFIX};

use RegisterOperation::{Read, Write};

type ProcessID = usize;

pub struct TimedAction<T> {
    process: ProcessID,
    action: Action<T>,
    happened_at: Instant,
}

impl<T> TimedAction<T> {
    fn new(process: ProcessID, action: Action<T>) -> Self {
        Self {
            process,
            action,
            happened_at: Instant::now(),
        }
    }
}

type RecordedAction<T> = TimedAction<RegisterOperation<T>>;
type RecordedResult<T> = Result<(RecordedAction<T>, RecordedAction<T>), Box<dyn Error>>;

// A Register client that records call and response information about the
// operations that it performs.
struct RecordingRegisterClient<T> {
    process: ProcessID,
    rng: ThreadRng,
    url: Uri,
    value_type: PhantomData<T>,
}

impl<T: Clone + DeserializeOwned + Serialize> RecordingRegisterClient<T>
where
    Standard: Distribution<T>,
{
    fn new(process: ProcessID, url: Uri) -> Self {
        Self {
            process,
            url,
            rng: thread_rng(),
            value_type: PhantomData,
        }
    }

    fn record(&self, action: Action<RegisterOperation<T>>) -> RecordedAction<T> {
        TimedAction::new(self.process, action)
    }

    async fn perform_random_operation(&mut self) -> RecordedResult<T> {
        let should_write: bool = self.rng.gen::<bool>();
        if should_write {
            let value: T = self.rng.gen::<T>();
            Ok(self.write(value).await?)
        } else {
            Ok(self.read().await?)
        }
    }

    async fn read(&self) -> RecordedResult<T> {
        let call_action = Action::Call(Read(None));
        let call = self.record(call_action);

        let result = get(self.url.clone()).await.unwrap();
        assert!(result.status().is_success());
        let body = result.collect().await?.aggregate();
        let value: T = serde_json::from_reader(body.reader())?;

        let response_action = Action::Response(Read(Some(value)));
        let response = self.record(response_action);
        Ok((call, response))
    }

    async fn write(&self, value: T) -> RecordedResult<T> {
        let call_action = Action::Call(Write(value.clone()));
        let call = self.record(call_action);

        let result = post(self.url.clone(), json!(value.clone())).await.unwrap();
        assert!(result.status().is_success());

        let response_action = Action::Response(Write(value));
        let response = self.record(response_action);
        Ok((call, response))
    }
}

/// Asserts that in a network where a random minority of servers are faulty, a
/// random sequence of reads and writes by correct clients will result in a
/// linearizable history.
#[test]
fn random_reads_and_writes_with_random_failures() {
    const NUM_CLIENTS: usize = 5;
    const NUM_OPERATIONS: usize = 20;
    const NUM_SERVERS: usize = 10;
    const FAILURE_RATE: f64 = 1.0;

    // Simulate a network where a random minority of servers
    // fail with non-zero probability.
    let mut sim = simulate_servers(NUM_SERVERS);
    let servers: Vec<String> = (0..NUM_SERVERS)
        .map(|i| format!("{SERVER_PREFIX}-{i}"))
        .collect();

    let mut rng = thread_rng();
    let minority = ((NUM_SERVERS as f32 / 2.0).ceil() - 1.0) as usize;

    let faulty_servers: Vec<String> = servers
        .clone()
        .into_iter()
        .choose_multiple(&mut rng, minority);
    let correct_servers: Vec<String> = servers
        .clone()
        .into_iter()
        .filter(|s| !faulty_servers.contains(s))
        .collect();

    // Set the failure rate for any connection involving a faulty server
    for faulty in faulty_servers {
        for server in servers.clone() {
            if faulty == server {
                continue;
            };
            let a = faulty.clone();
            let b = server.clone();
            sim.set_link_fail_rate(a, b, FAILURE_RATE);
        }
    }

    let actions: Arc<Mutex<Vec<TimedAction<RegisterOperation<u32>>>>> =
        Arc::new(Mutex::new(vec![]));

    // Simulate clients that submit requests to correct servers.
    assert!(NUM_CLIENTS <= correct_servers.len());
    for i in 0..NUM_CLIENTS {
        let client_name = format!("client-{i}");
        let server_name = correct_servers.iter().choose(&mut rng).unwrap();
        let url: Uri = format!("http://{server_name}:9999/register")
            .parse()
            .unwrap();

        let actions = actions.clone();
        sim.client(client_name, async move {
            let mut client = RecordingRegisterClient::<u32>::new(i, url);
            for _ in 0..NUM_OPERATIONS {
                let (call, response) = client.perform_random_operation().await?;
                let mut actions = actions.lock().unwrap();
                actions.push(call);
                actions.push(response);
            }
            Ok(())
        });
    }

    sim.run().unwrap();

    // Collect log of call/response actions that occured during the simulation
    // and assert that the resulting history is linearizable
    let actions = &mut *actions.lock().unwrap();
    actions.sort_by(|a, b| a.happened_at.cmp(&b.happened_at));
    let history = History::from_actions(
        actions
            .iter()
            .map(|ta| (ta.process, ta.action.clone()))
            .collect(),
    );
    assert!(WLGChecker::is_linearizable(
        RegisterSpecification::new(),
        history
    ));
}
