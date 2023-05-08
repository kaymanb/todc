use std::collections::HashMap;
use std::hash::Hash;
use std::marker::{Send, Sync};
use std::sync::Arc;
use std::thread;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use todc_sm::snapshot::aad_plus_93::{
    BoundedAtomicSnapshot, BoundedMutexSnapshot, UnboundedAtomicSnapshot, UnboundedMutexSnapshot,
};
use todc_sm::snapshot::ar_98::LatticeMutexSnapshot;
use todc_sm::snapshot::mutex::MutexSnapshot;
use todc_sm::snapshot::Snapshot;

const MIN_NUM_THREADS: usize = 2;
const MAX_NUM_THREADS: usize = 5;

#[derive(Hash, PartialEq, Eq)]
enum SnapshotName {
    BoundedAtomic,
    BoundedMutex,
    LatticeMutex,
    Mutex,
    UnboundedAtomic,
    UnboundedMutex,
}

// TODO: Deal with const generics in a better way...
use SnapshotType::*;
enum SnapshotType {
    // MutexSnapshot
    MutexTwo(Arc<MutexSnapshot<u8, 2>>),
    MutexThree(Arc<MutexSnapshot<u8, 3>>),
    MutexFour(Arc<MutexSnapshot<u8, 4>>),
    MutexFive(Arc<MutexSnapshot<u8, 5>>),
    // UnboundedAtomicSnapshot
    UnboundedAtomicTwo(Arc<UnboundedAtomicSnapshot<2>>),
    UnboundedAtomicThree(Arc<UnboundedAtomicSnapshot<3>>),
    UnboundedAtomicFour(Arc<UnboundedAtomicSnapshot<4>>),
    UnboundedAtomicFive(Arc<UnboundedAtomicSnapshot<5>>),
    // UnboundedMutexSnapshot
    UnboundedMutexTwo(Arc<UnboundedMutexSnapshot<u8, 2>>),
    UnboundedMutexThree(Arc<UnboundedMutexSnapshot<u8, 3>>),
    UnboundedMutexFour(Arc<UnboundedMutexSnapshot<u8, 4>>),
    UnboundedMutexFive(Arc<UnboundedMutexSnapshot<u8, 5>>),
    // BoundedAtomicSnapshot
    BoundedAtomicTwo(Arc<BoundedAtomicSnapshot<2>>),
    BoundedAtomicThree(Arc<BoundedAtomicSnapshot<3>>),
    BoundedAtomicFour(Arc<BoundedAtomicSnapshot<4>>),
    BoundedAtomicFive(Arc<BoundedAtomicSnapshot<5>>),
    // BoundedMutexSnapshot
    BoundedMutexTwo(Arc<BoundedMutexSnapshot<u8, 2>>),
    BoundedMutexThree(Arc<BoundedMutexSnapshot<u8, 3>>),
    BoundedMutexFour(Arc<BoundedMutexSnapshot<u8, 4>>),
    BoundedMutexFive(Arc<BoundedMutexSnapshot<u8, 5>>),
    // LatticeMutexSnapshot
    LatticeMutexTwo(Arc<LatticeMutexSnapshot<u8, 2, 256>>),
    LatticeMutexThree(Arc<LatticeMutexSnapshot<u8, 3, 256>>),
    LatticeMutexFour(Arc<LatticeMutexSnapshot<u8, 4, 256>>),
    LatticeMutexFive(Arc<LatticeMutexSnapshot<u8, 5, 256>>),
}

fn do_updates_and_scans<const N: usize, S: Snapshot<N, Value = u8> + Send + Sync + 'static>(
    snapshot: &Arc<S>,
    num_threads: usize,
) {
    let mut handles = Vec::new();

    for i in 0..num_threads {
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

fn benchmark_snapshots(
    snapshots: &HashMap<(SnapshotName, usize), SnapshotType>,
    name: SnapshotName,
    num_threads: usize,
) {
    let snapshot = snapshots.get(&(name, num_threads)).unwrap();
    match snapshot {
        // MutexSnapshot
        MutexTwo(snapshot) => do_updates_and_scans(snapshot, num_threads),
        MutexThree(snapshot) => do_updates_and_scans(snapshot, num_threads),
        MutexFour(snapshot) => do_updates_and_scans(snapshot, num_threads),
        MutexFive(snapshot) => do_updates_and_scans(snapshot, num_threads),
        // UnboundedAtomicSnapshot
        UnboundedAtomicTwo(snapshot) => do_updates_and_scans(snapshot, num_threads),
        UnboundedAtomicThree(snapshot) => do_updates_and_scans(snapshot, num_threads),
        UnboundedAtomicFour(snapshot) => do_updates_and_scans(snapshot, num_threads),
        UnboundedAtomicFive(snapshot) => do_updates_and_scans(snapshot, num_threads),
        // UnboundedMutexSnapshot
        UnboundedMutexTwo(snapshot) => do_updates_and_scans(snapshot, num_threads),
        UnboundedMutexThree(snapshot) => do_updates_and_scans(snapshot, num_threads),
        UnboundedMutexFour(snapshot) => do_updates_and_scans(snapshot, num_threads),
        UnboundedMutexFive(snapshot) => do_updates_and_scans(snapshot, num_threads),
        // BoundedAtomicSnapshot
        BoundedAtomicTwo(snapshot) => do_updates_and_scans(snapshot, num_threads),
        BoundedAtomicThree(snapshot) => do_updates_and_scans(snapshot, num_threads),
        BoundedAtomicFour(snapshot) => do_updates_and_scans(snapshot, num_threads),
        BoundedAtomicFive(snapshot) => do_updates_and_scans(snapshot, num_threads),
        // BoundedMutexSnapshot
        BoundedMutexTwo(snapshot) => do_updates_and_scans(snapshot, num_threads),
        BoundedMutexThree(snapshot) => do_updates_and_scans(snapshot, num_threads),
        BoundedMutexFour(snapshot) => do_updates_and_scans(snapshot, num_threads),
        BoundedMutexFive(snapshot) => do_updates_and_scans(snapshot, num_threads),
        // LatticeMutexSnapshot
        LatticeMutexTwo(snapshot) => do_updates_and_scans(snapshot, num_threads),
        LatticeMutexThree(snapshot) => do_updates_and_scans(snapshot, num_threads),
        LatticeMutexFour(snapshot) => do_updates_and_scans(snapshot, num_threads),
        LatticeMutexFive(snapshot) => do_updates_and_scans(snapshot, num_threads),
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Snapshots");

    let snapshots: HashMap<(SnapshotName, usize), SnapshotType> = HashMap::from([
        // MutexSnapshot
        (
            (SnapshotName::Mutex, 2),
            MutexTwo(Arc::new(MutexSnapshot::new())),
        ),
        (
            (SnapshotName::Mutex, 3),
            MutexThree(Arc::new(MutexSnapshot::new())),
        ),
        (
            (SnapshotName::Mutex, 4),
            MutexFour(Arc::new(MutexSnapshot::new())),
        ),
        (
            (SnapshotName::Mutex, 5),
            MutexFive(Arc::new(MutexSnapshot::new())),
        ),
        // UnboundedAtomicSnapshot
        (
            (SnapshotName::UnboundedAtomic, 2),
            UnboundedAtomicTwo(Arc::new(UnboundedAtomicSnapshot::new())),
        ),
        (
            (SnapshotName::UnboundedAtomic, 3),
            UnboundedAtomicThree(Arc::new(UnboundedAtomicSnapshot::new())),
        ),
        (
            (SnapshotName::UnboundedAtomic, 4),
            UnboundedAtomicFour(Arc::new(UnboundedAtomicSnapshot::new())),
        ),
        (
            (SnapshotName::UnboundedAtomic, 5),
            UnboundedAtomicFive(Arc::new(UnboundedAtomicSnapshot::new())),
        ),
        // UnboundedMutexSnapshot
        (
            (SnapshotName::UnboundedMutex, 2),
            UnboundedMutexTwo(Arc::new(UnboundedMutexSnapshot::new())),
        ),
        (
            (SnapshotName::UnboundedMutex, 3),
            UnboundedMutexThree(Arc::new(UnboundedMutexSnapshot::new())),
        ),
        (
            (SnapshotName::UnboundedMutex, 4),
            UnboundedMutexFour(Arc::new(UnboundedMutexSnapshot::new())),
        ),
        (
            (SnapshotName::UnboundedMutex, 5),
            UnboundedMutexFive(Arc::new(UnboundedMutexSnapshot::new())),
        ),
        // BoundedAtomicSnapshot
        (
            (SnapshotName::BoundedAtomic, 2),
            BoundedAtomicTwo(Arc::new(BoundedAtomicSnapshot::new())),
        ),
        (
            (SnapshotName::BoundedAtomic, 3),
            BoundedAtomicThree(Arc::new(BoundedAtomicSnapshot::new())),
        ),
        (
            (SnapshotName::BoundedAtomic, 4),
            BoundedAtomicFour(Arc::new(BoundedAtomicSnapshot::new())),
        ),
        (
            (SnapshotName::BoundedAtomic, 5),
            BoundedAtomicFive(Arc::new(BoundedAtomicSnapshot::new())),
        ),
        // BoundedMutexSnapshot
        (
            (SnapshotName::BoundedMutex, 2),
            BoundedMutexTwo(Arc::new(BoundedMutexSnapshot::new())),
        ),
        (
            (SnapshotName::BoundedMutex, 3),
            BoundedMutexThree(Arc::new(BoundedMutexSnapshot::new())),
        ),
        (
            (SnapshotName::BoundedMutex, 4),
            BoundedMutexFour(Arc::new(BoundedMutexSnapshot::new())),
        ),
        (
            (SnapshotName::BoundedMutex, 5),
            BoundedMutexFive(Arc::new(BoundedMutexSnapshot::new())),
        ),
        // LatticeMutexSnapshot
        (
            (SnapshotName::LatticeMutex, 2),
            LatticeMutexTwo(Arc::new(LatticeMutexSnapshot::new())),
        ),
        (
            (SnapshotName::LatticeMutex, 3),
            LatticeMutexThree(Arc::new(LatticeMutexSnapshot::new())),
        ),
        (
            (SnapshotName::LatticeMutex, 4),
            LatticeMutexFour(Arc::new(LatticeMutexSnapshot::new())),
        ),
        (
            (SnapshotName::LatticeMutex, 5),
            LatticeMutexFive(Arc::new(LatticeMutexSnapshot::new())),
        ),
    ]);

    for n in MIN_NUM_THREADS..MAX_NUM_THREADS + 1 {
        group.bench_with_input(BenchmarkId::new("Mutex", n), &n, |b, n| {
            b.iter(|| benchmark_snapshots(&snapshots, SnapshotName::Mutex, *n))
        });
        group.bench_with_input(BenchmarkId::new("AAD+93/UnboundedAtomic", n), &n, |b, n| {
            b.iter(|| benchmark_snapshots(&snapshots, SnapshotName::UnboundedAtomic, *n))
        });
        group.bench_with_input(BenchmarkId::new("AAD+93/UnboundedMutex", n), &n, |b, n| {
            b.iter(|| benchmark_snapshots(&snapshots, SnapshotName::UnboundedMutex, *n))
        });
        group.bench_with_input(BenchmarkId::new("AAD+93/BoundedAtomic", n), &n, |b, n| {
            b.iter(|| benchmark_snapshots(&snapshots, SnapshotName::BoundedAtomic, *n))
        });
        group.bench_with_input(BenchmarkId::new("AAD+93/BoundedMutex", n), &n, |b, n| {
            b.iter(|| benchmark_snapshots(&snapshots, SnapshotName::BoundedMutex, *n))
        });
        group.bench_with_input(BenchmarkId::new("AR98/LatticeMutex", n), &n, |b, n| {
            b.iter(|| benchmark_snapshots(&snapshots, SnapshotName::LatticeMutex, *n))
        });
    }
}

criterion_group! {
    all_snapshot_implementations,
    criterion_benchmark,
}
criterion_main! {
    all_snapshot_implementations
}
