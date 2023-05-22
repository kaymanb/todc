use std::error::Error;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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

#[derive(Debug)]
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
type EmptyResult = Result<(), Box<dyn Error>>;

/// Asserts that the sequence of actions corresponds to a linearizable
/// history of register operations.
///
/// # Panics
///
/// Panics if the history of register operations are is not linearizable.
fn assert_linearizable<T>(mut actions: Vec<RecordedAction<T>>)
where
    T: Clone + Debug + Default + Eq + Hash,
{
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

/// A Register client that records call and response information about the
/// operations that it performs.
struct RecordingRegisterClient<T> {
    actions: Arc<Mutex<Vec<RecordedAction<T>>>>,
    process: ProcessID,
    rng: ThreadRng,
    url: Uri,
    value_type: PhantomData<T>,
}

impl<T: Clone + DeserializeOwned + Serialize> RecordingRegisterClient<T>
where
    Standard: Distribution<T>,
{
    fn new(process: ProcessID, url: Uri, actions: Arc<Mutex<Vec<RecordedAction<T>>>>) -> Self {
        Self {
            actions,
            process,
            url,
            rng: thread_rng(),
            value_type: PhantomData,
        }
    }

    fn record(&self, action: Action<RegisterOperation<T>>) {
        let timed_action = TimedAction::new(self.process, action);
        let mut actions = self.actions.lock().unwrap();
        actions.push(timed_action);
    }

    async fn perform_random_operation(&mut self, p: f64) -> EmptyResult {
        let should_write: bool = self.rng.gen_bool(p);
        if should_write {
            let value: T = self.rng.gen::<T>();
            self.write(value).await
        } else {
            self.read().await
        }
    }

    async fn read(&self) -> EmptyResult {
        let call_action = Action::Call(Read(None));
        self.record(call_action);

        let result = get(self.url.clone()).await.unwrap();
        assert!(result.status().is_success());
        let body = result.collect().await?.aggregate();
        let value: T = serde_json::from_reader(body.reader())?;

        let response_action = Action::Response(Read(Some(value)));
        self.record(response_action);
        Ok(())
    }

    async fn write(&self, value: T) -> EmptyResult {
        let call_action = Action::Call(Write(value.clone()));
        self.record(call_action);

        let result = post(self.url.clone(), json!(value.clone())).await.unwrap();
        assert!(result.status().is_success());

        let response_action = Action::Response(Write(value));
        self.record(response_action);
        Ok(())
    }
}

/// Asserts that in a network where a random minority of servers are faulty, a
/// random sequence of reads and writes by correct clients will result in a
/// linearizable history.
#[test]
fn random_reads_and_writes_with_random_failures() {
    const NUM_CLIENTS: usize = 5;
    const NUM_OPERATIONS: usize = 25;
    const NUM_SERVERS: usize = 10;
    const WRITE_PROBABILITY: f64 = 1.0 / 2.0;
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
            let mut client = RecordingRegisterClient::<u32>::new(i, url, actions);
            for _ in 0..NUM_OPERATIONS {
                client.perform_random_operation(WRITE_PROBABILITY).await?;
            }
            Ok(())
        });
    }

    sim.run().unwrap();

    // Collect log of call/response actions that occured during the simulation
    // and assert that the resulting history is linearizable
    let actions = Arc::try_unwrap(actions).unwrap().into_inner().unwrap();
    assert_linearizable(actions);
}

/// Asserts that a pair of reads that are concurrent with a write will return values
/// that are part of a linearizable history.
///
/// This particular scenario is alluded to by Attiya, Bar-Noy, and Dolev
/// [[ABD95]](https://dl.acm.org/doi/pdf/10.1145/200836.200869) when providing
/// intuition for why a `read` operation must announce its values to all others prior
/// to returning.
///
/// > Informally, this announcement is needed since, otherwise, it is possible
/// > for a read operation to return the label of a write operation that is
/// > concurrent with it, and for a later read operation to return an earlier label.
///
/// In the test, we manipulate the network in order to simulate a scenario
/// where the first read _does_ return the value (i.e. label) of the write
/// operation it is concurrent with, and the linearizability assertion will
/// implicitly check that the subsequent read does as well.
#[test]
fn pair_of_reads_with_concurrent_write() {
    const NUM_SERVERS: usize = 5;

    let mut sim = simulate_servers(NUM_SERVERS);
    sim.set_max_message_latency(Duration::from_millis(1));

    let actions: Arc<Mutex<Vec<TimedAction<RegisterOperation<u32>>>>> =
        Arc::new(Mutex::new(vec![]));

    let actions_clone = actions.clone();
    sim.client("concurrent-writer", async move {
        let url: Uri = format!("http://server-0:9999/register").parse().unwrap();
        let client = RecordingRegisterClient::<u32>::new(0, url, actions_clone);

        // Initially, server-0 is isolated in the network. In particular,
        // server-1 and server-2 will not recieve information about the written
        // value until their messages are released.
        turmoil::hold("server-0", "server-1");
        turmoil::hold("server-0", "server-2");
        turmoil::hold("server-0", "server-3");
        turmoil::hold("server-0", "server-4");

        client.write(123).await?;
        Ok(())
    });

    let actions_clone = actions.clone();
    sim.client("reader", async move {
        // First, release messages between server-0 to server-1 and wait for
        // any that occur during the concurrent write to be delivered.
        turmoil::release("server-0", "server-1");
        std::thread::sleep(Duration::from_secs(1));

        // Then, perform a read on server-1.
        let url: Uri = format!("http://server-1:9999/register").parse().unwrap();
        let client_1 = RecordingRegisterClient::<u32>::new(1, url, actions_clone.clone());
        client_1.read().await?;

        // Next, hold messages between server-1 and server-2, preventing server-2
        // from asking for information about the value returned by the ealier read.
        turmoil::hold("server-1", "server-2");

        // Perform a read on server-2.
        let url: Uri = format!("http://server-2:9999/register").parse().unwrap();
        let client_2 = RecordingRegisterClient::<u32>::new(2, url, actions_clone);
        client_2.read().await?;

        // Finally, release all messeges from server-0, allow the write to complete.
        turmoil::release("server-0", "server-2");
        turmoil::release("server-0", "server-3");
        turmoil::release("server-0", "server-4");
        Ok(())
    });

    sim.run().unwrap();

    let actions = Arc::try_unwrap(actions).unwrap().into_inner().unwrap();
    assert_linearizable(actions);
}
