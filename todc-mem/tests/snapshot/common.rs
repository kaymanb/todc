use std::fmt::Debug;
use std::hash::Hash;
use std::marker::{Send, Sync};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rand::distributions::Standard;
use rand::prelude::Distribution;
use shuttle::rand::{rngs::ThreadRng, thread_rng, Rng};
use shuttle::thread;
use todc_mem::snapshot::Snapshot;
use todc_utils::specifications::snapshot::{ProcessId, SnapshotOperation, SnapshotSpecification};
use todc_utils::{Action, History, WGLChecker};

pub const NUM_ITERATIONS: usize = 250;
pub const NUM_OPERATIONS: usize = 50;
pub const NUM_PREEMPTIONS: usize = 3;
pub const NUM_THREADS: usize = 5;

#[derive(Debug, Clone)]
pub struct TimedAction<T, const N: usize> {
    process: ProcessId,
    action: Action<SnapshotOperation<T, N>>,
    happened_at: Instant,
}

impl<T, const N: usize> TimedAction<T, N> {
    fn new(process: ProcessId, action: Action<SnapshotOperation<T, N>>) -> Self {
        Self {
            process,
            action,
            happened_at: Instant::now(),
        }
    }
}

/// Asserts that the sequence of actions corresponds to a linearizable
/// history of snapshot operations.
///
/// # Panics
///
/// Panics if the history of snapshot actions is not linearizable.
fn assert_linearizable<T, const N: usize>(mut actions: Vec<TimedAction<T, N>>)
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

    assert!(WGLChecker::<SnapshotSpecification<T, N>>::is_linearizable(
        history
    ));
}

/// A snapshot that records metadata about operations performed on it.
pub struct RecordingSnapshot<const N: usize, S: Snapshot<{ N }>> {
    actions: Arc<Mutex<Vec<TimedAction<S::Value, N>>>>,
    snapshot: S,
}

impl<const N: usize, S: Snapshot<{ N }>> RecordingSnapshot<N, S>
where
    Standard: Distribution<S::Value>,
{
    pub fn new() -> Self {
        Self {
            actions: Arc::new(Mutex::new(vec![])),
            snapshot: S::new(),
        }
    }

    pub fn perform_random_operation(&self, i: ProcessId, p: f64, rng: &mut ThreadRng) {
        let should_update: bool = rng.gen_bool(p);
        if should_update {
            let value = rng.gen::<S::Value>();
            self.update(i, value);
        } else {
            self.scan(i);
        }
    }

    fn record(&self, i: ProcessId, action: Action<SnapshotOperation<S::Value, N>>) {
        let timed_action = TimedAction::new(i, action);
        let mut actions = self.actions.lock().unwrap();
        actions.push(timed_action);
    }

    pub fn scan(&self, i: ProcessId) {
        let call = Action::Call(SnapshotOperation::Scan(i, None));
        self.record(i, call);

        let view = self.snapshot.scan(i);

        let response = Action::Response(SnapshotOperation::Scan(i, Some(view)));
        self.record(i, response);
    }

    pub fn update(&self, i: ProcessId, value: S::Value) {
        let call = Action::Call(SnapshotOperation::Update(i, value.clone()));
        self.record(i, call);

        self.snapshot.update(i, value.clone());

        let response = Action::Response(SnapshotOperation::Update(i, value.clone()));
        self.record(i, response);
    }
}

/// Assert that a history consisting of a random sequence of snapshot
/// operations is linearizable.
///
/// # Panics
///
/// Panics if the history of random snapshot operations is not linearizable.
pub fn assert_random_operations_are_linearizable<
    const N: usize,
    S: Snapshot<{ N }> + 'static + Send + Sync,
>()
where
    Standard: Distribution<S::Value>,
    S::Value: Clone + Debug + Default + Eq + Hash + Send,
{
    const SCAN_PROBABILITY: f64 = 1.0 / 2.0;

    let mut handles = Vec::new();
    let snapshot: Arc<RecordingSnapshot<N, S>> = Arc::new(RecordingSnapshot::new());

    for i in 0..N {
        let snapshot = snapshot.clone();
        handles.push(thread::spawn(move || {
            let mut rng = thread_rng();
            for _ in 0..NUM_OPERATIONS {
                snapshot.perform_random_operation(i, SCAN_PROBABILITY, &mut rng);
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let actions = snapshot.actions.lock().unwrap().clone();
    assert_linearizable(actions);
}
