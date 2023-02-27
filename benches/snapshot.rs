use std::marker::{Send, Sync};
use std::sync::Arc;
use std::thread;

use criterion::{criterion_group, criterion_main, Criterion};

use todc::snapshot::aad_plus_93::{BoundedSnapshot, UnboundedSnapshot};
use todc::snapshot::ar_98::LatticeSnapshot;
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

    let unbounded: Arc<UnboundedSnapshot<usize, NUM_THREADS>> = Arc::new(UnboundedSnapshot::new());
    let bounded: Arc<BoundedSnapshot<usize, NUM_THREADS>> = Arc::new(BoundedSnapshot::new());
    let fast: Arc<LatticeSnapshot<usize, NUM_THREADS, 256>> = Arc::new(LatticeSnapshot::new());
    group.bench_function("AAD+93 - Unbounded", |b| {
        b.iter(|| do_updates_and_scans(&unbounded))
    });
    group.bench_function("AAD+93 - Bounded", |b| {
        b.iter(|| do_updates_and_scans(&bounded))
    });
    group.bench_function("AR98 - Lattice", |b| b.iter(|| do_updates_and_scans(&fast)));
}

criterion_group! {
    all_implementations,
    criterion_benchmark
}
criterion_main! {
    all_implementations
}
