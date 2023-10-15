use std::error::Error;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rand::distributions::Standard;
use rand::prelude::Distribution;
use rand::rngs::StdRng;
use rand::seq::IteratorRandom;
use rand::{thread_rng, Rng, SeedableRng};
use serde::de::DeserializeOwned;
use serde::Serialize;

use todc_net::register::abd_95::AtomicRegister;
use todc_utils::specifications::register::{RegisterOperation, RegisterSpecification};
use todc_utils::{Action, History, WGLChecker};

use crate::register::abd_95::common::{simulate_servers_with_seed, SERVER_PREFIX};

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
/// Panics if the history of register operations is not linearizable.
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
    assert!(WGLChecker::<RegisterSpecification<T>>::is_linearizable(
        history
    ));
}

/// A Register client that records call and response information about the
/// operations that it performs.
struct RecordingRegisterClient<T: Clone + Debug + Default + DeserializeOwned + Ord + Send> {
    actions: Arc<Mutex<Vec<RecordedAction<T>>>>,
    process: ProcessID,
    register: AtomicRegister<T>,
    rng: StdRng,
    value_type: PhantomData<T>,
}

impl<T: Debug + Default + Clone + DeserializeOwned + Ord + Send + Serialize + 'static>
    RecordingRegisterClient<T>
where
    Standard: Distribution<T>,
{
    fn new(
        process: ProcessID,
        register: AtomicRegister<T>,
        rng: StdRng,
        actions: Arc<Mutex<Vec<RecordedAction<T>>>>,
    ) -> Self {
        Self {
            actions,
            process,
            register,
            rng,
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
            self.read().await?;
            Ok(())
        }
    }

    async fn read(&self) -> Result<T, Box<dyn Error>> {
        let call_action = Action::Call(Read(None));
        self.record(call_action);

        let value = self.register.read().await.unwrap();

        let response_action = Action::Response(Read(Some(value.clone())));
        self.record(response_action);
        Ok(value)
    }

    async fn write(&self, value: T) -> EmptyResult {
        let call_action = Action::Call(Write(value.clone()));
        self.record(call_action);

        self.register.write(value.clone()).await.unwrap();

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
    // HACK: Run fewer iterations when calculating code coverage.
    #[cfg(coverage)]
    const NUM_CLIENTS: usize = 3;
    #[cfg(coverage)]
    const NUM_OPERATIONS: usize = 10;
    #[cfg(coverage)]
    const NUM_SERVERS: usize = 6;

    #[cfg(not(coverage))]
    const NUM_CLIENTS: usize = 10;
    #[cfg(not(coverage))]
    const NUM_OPERATIONS: usize = 100;
    #[cfg(not(coverage))]
    const NUM_SERVERS: usize = 20;

    const WRITE_PROBABILITY: f64 = 1.0 / 2.0;
    const FAILURE_RATE: f64 = 0.8;

    // Simulate a network where a random minority of servers
    // fail with non-zero probability.
    let (mut sim, registers, seed) = simulate_servers_with_seed(NUM_SERVERS);
    let servers: Vec<String> = (0..NUM_SERVERS)
        .map(|i| format!("{SERVER_PREFIX}-{i}"))
        .collect();

    let mut rng = StdRng::seed_from_u64(seed);
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

    // Simulate clients that submit requests.
    assert!(NUM_CLIENTS <= correct_servers.len());
    for (i, register) in registers.into_iter().enumerate().take(NUM_CLIENTS) {
        let actions = actions.clone();
        let rng = rng.clone();
        let client_name = format!("client-{i}");
        sim.client(client_name, async move {
            let mut client = RecordingRegisterClient::<u32>::new(i, register.clone(), rng, actions);
            for _ in 0..NUM_OPERATIONS {
                client.perform_random_operation(WRITE_PROBABILITY).await?;
            }
            Ok(())
        });
    }

    sim.run().unwrap();

    // Print the seed to enable re-trying a failed test.
    println!("This test used the random seed: {seed}");

    // Collect log of call/response actions that occured during the simulation
    // and assert that the resulting history is linearizable
    let actions = Arc::try_unwrap(actions).unwrap().into_inner().unwrap();
    assert_linearizable(actions);
}
