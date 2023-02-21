use std::fmt::Debug;
use std::time::Instant;

use todc::snapshot::Snapshot;

#[derive(Debug)]
pub enum Op {
    Update,
    Scan,
}

#[derive(Debug)]
pub struct Event<T> {
    process: usize,
    pub start: Instant,
    end: Instant,
    op: Op,
    result: Option<T>,
}

pub struct HistoriedSnapshot<const N: usize, S: Snapshot<{ N }>> {
    snapshot: S,
}

impl<const N: usize, S: Snapshot<{ N }>> HistoriedSnapshot<N, S> {
    pub fn new() -> Self {
        Self { snapshot: S::new() }
    }

    pub fn scan(&self, i: usize) -> Event<[S::Value; N]> {
        let start = Instant::now();
        let result = Some(self.snapshot.scan(i));
        let end = Instant::now();
        Event {
            process: i,
            start,
            end,
            op: Op::Scan,
            result,
        }
    }

    pub fn update(&self, i: usize, value: S::Value) -> Event<[S::Value; N]> {
        let start = Instant::now();
        self.snapshot.update(i, value);
        let end = Instant::now();
        Event {
            process: i,
            start,
            end,
            op: Op::Update,
            result: None,
        }
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
