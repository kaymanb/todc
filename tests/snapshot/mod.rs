use std::time::Instant;

use todc::snapshot::Snapshot;
use todc::linearizability::history::Action;
use utils::specifications::snapshot::{ProcessID, SnapshotOperation};

#[cfg(loom)]
mod aad_plus_93;

#[cfg(loom)]
mod ar_98;

pub struct TimedAction<T, const N: usize> {
    process: ProcessID,
    action: Action<SnapshotOperation<T, N>>,
    happened_at: Instant,
}

impl<T, const N: usize> TimedAction<T, N> {
    fn new(process: ProcessID, action: Action<SnapshotOperation<T, N>>) -> Self {
        Self {
            process,
            action,
            happened_at: Instant::now()
        }
    }
}

pub struct RecordingSnapshot<const N: usize, S: Snapshot<{ N }>> {
    snapshot: S,
}

impl<const N: usize, S: Snapshot<{ N }>> RecordingSnapshot<N, S> {
    pub fn new() -> Self {
        Self { snapshot: S::new() }
    }

    pub fn scan(&self, i: usize) -> (TimedAction<S::Value, N>, TimedAction<S::Value, N>) {
        let call = TimedAction::new(
            i, Action::Call(SnapshotOperation::Scan(i, None))
        );
        let view = self.snapshot.scan(i);
        let response = TimedAction::new(
            i, Action::Response(SnapshotOperation::Scan(i, Some(view)))
        );
        (call, response)
    }

    pub fn update(&self, i: usize, value: S::Value) -> (TimedAction<S::Value, N>, TimedAction<S::Value, N>) {
        let call = TimedAction::new(
            i, Action::Call(SnapshotOperation::Update(i, value.clone()))
        );
        self.snapshot.update(i, value.clone());
        let response = TimedAction::new(
            i, Action::Response(SnapshotOperation::Update(i, value.clone()))
        );
        (call, response)
    }
}

