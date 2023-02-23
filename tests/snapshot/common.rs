use std::fmt::Debug;
use std::time::Instant;

use todc::snapshot::Snapshot;
use utils::specifications::snapshot::{ProcessID, SnapshotOperation};

pub struct Action<T, N> {
    process: ProcessID,
    operation: SnapshotOperation<T, N>,
    happened_at: Instant,
}

pub struct RecordingSnapshot<const N: usize, S: Snapshot<{ N }>> {
    snapshot: S,
}

impl<const N: usize, S: Snapshot<{ N }>> HistoriedSnapshot<N, S> {
    pub fn new() -> Self {
        Self { snapshot: S::new() }
    }

    pub fn scan(&self, i: usize) -> (Action<S::Value, N>, Action<S::Value, N>) {
        let call = Action {
            process: i,
            operation: SnapshotOperation::Scan(i, None),
            happened_at: Instant::now()
        };
        let view = self.snapshot.scan(i);
        let response = Action {
            process: i,
            operation: SnapshotOperation::Scan(i, Some(view)),
            happened_at: Instant::now()
        };
        (call, response)
    }

    pub fn update(&self, i: usize, value: S::Value) -> (Action<S::Value, N>, Action<S::Value, N>) {
        let call = Action {
            process: i,
            operation: SnapshotOperation::Update(i, value),
            happened_at: Instant::now()
        };
        self.snapshot.update(i, value);
        let response = Action {
            process: i,
            operation: SnapshotOperation::Update(i, value),
            happened_at: Instant::now()
        };
        (call, response)
    }
}

/// Assert that at least one process performed it's scan() operation last,
/// and recieved a view with no empty components.
pub fn assert_maximal_view_exists<T: Default + PartialEq, const N: usize>(views: &Vec<[T; N]>) {
    assert!(views
        .iter()
        .any(|view| view.iter().all(|val| *val != T::default())));
}

/// Assert that, for any pair of views V1 and V2, if both V1 and V2 are non-empty
/// in component i, then their values in that component are equal.
pub fn assert_views_are_comparable<T: Debug + Default + PartialEq, const N: usize>(
    views: &Vec<[T; N]>,
) {
    for v1 in views {
        for v2 in views {
            for i in 0..N {
                if v1[i] != T::default() && v2[i] != T::default() {
                    assert_eq!(v1[i], v2[i])
                }
            }
        }
    }
}
