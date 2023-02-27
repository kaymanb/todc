use std::marker::{Send, Sync};
use std::sync::Arc;
use std::thread;

use criterion::{criterion_group, criterion_main, Criterion};

use todc::snapshot::aad_plus_93::{BoundedAtomicSnapshot, UnboundedAtomicSnapshot};
use todc::snapshot::ar_98::AtomicSnapshot;
use todc::snapshot::Snapshot;

const NUM_THREADS: usize = 3;

fn do_updates_and_scans<const N: usize, S: Snapshot<N, Value = usize> + Send + Sync + 'static>(
    snapshot: &Arc<S>,
) {
    let mut handles = Vec::new();

    for i in 0..NUM_THREADS {
        let snapshot = snapshot.clone();
        handles.push(thread::spawn(move || {
            for j in 0..100 {
                snapshot.update(i, j);
                snapshot.scan(i);
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Snapshots");

    let unbounded: Arc<UnboundedAtomicSnapshot<usize, NUM_THREADS>> =
        Arc::new(UnboundedAtomicSnapshot::new());
    let bounded: Arc<BoundedAtomicSnapshot<usize, NUM_THREADS>> =
        Arc::new(BoundedAtomicSnapshot::new());
    let fast: Arc<AtomicSnapshot<usize, NUM_THREADS, 256>> = Arc::new(AtomicSnapshot::new());
    group.bench_function("AAD+93 - Unbounded", |b| {
        b.iter(|| do_updates_and_scans(&unbounded))
    });
    group.bench_function("AAD+93 - Bounded", |b| {
        b.iter(|| do_updates_and_scans(&bounded))
    });
    group.bench_function("AR98", |b| b.iter(|| do_updates_and_scans(&fast)));
}

criterion_group! {
    all_implementations,
    criterion_benchmark
}
criterion_main! {
    all_implementations
}
