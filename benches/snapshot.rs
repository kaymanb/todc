use std::collections::HashMap;
use std::hash::Hash;
use std::marker::{Send, Sync};
use std::sync::Arc;
use std::thread;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use todc::snapshot::aad_plus_93::{
    BoundedSnapshot, UnboundedAtomicSnapshot, UnboundedMutexSnapshot,
};
use todc::snapshot::ar_98::LatticeSnapshot;
use todc::snapshot::Snapshot;

const MIN_NUM_THREADS: usize = 2;
const MAX_NUM_THREADS: usize = 5;

#[derive(Hash, PartialEq, Eq)]
enum SnapshotName {
    UnboundedAtomic,
    UnboundedMutex,
    Bounded,
    Lattice,
}

// TODO: Deal with const generics in a better way...
use SnapshotType::*;
enum SnapshotType {
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
    // BoundedSnapshot
    BoundedTwo(Arc<BoundedSnapshot<u8, 2>>),
    BoundedThree(Arc<BoundedSnapshot<u8, 3>>),
    BoundedFour(Arc<BoundedSnapshot<u8, 4>>),
    BoundedFive(Arc<BoundedSnapshot<u8, 5>>),
    // LatticeSnapshot
    LatticeTwo(Arc<LatticeSnapshot<u8, 2, 256>>),
    LatticeThree(Arc<LatticeSnapshot<u8, 3, 256>>),
    LatticeFour(Arc<LatticeSnapshot<u8, 4, 256>>),
    LatticeFive(Arc<LatticeSnapshot<u8, 5, 256>>),
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
        // BoundedSnapshot
        BoundedTwo(snapshot) => do_updates_and_scans(snapshot, num_threads),
        BoundedThree(snapshot) => do_updates_and_scans(snapshot, num_threads),
        BoundedFour(snapshot) => do_updates_and_scans(snapshot, num_threads),
        BoundedFive(snapshot) => do_updates_and_scans(snapshot, num_threads),
        // LatticeSnapshot
        LatticeTwo(snapshot) => do_updates_and_scans(snapshot, num_threads),
        LatticeThree(snapshot) => do_updates_and_scans(snapshot, num_threads),
        LatticeFour(snapshot) => do_updates_and_scans(snapshot, num_threads),
        LatticeFive(snapshot) => do_updates_and_scans(snapshot, num_threads),
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Snapshots");

    let snapshots: HashMap<(SnapshotName, usize), SnapshotType> = HashMap::from([
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
        // BoundedSnapshot
        (
            (SnapshotName::Bounded, 2),
            BoundedTwo(Arc::new(BoundedSnapshot::new())),
        ),
        (
            (SnapshotName::Bounded, 3),
            BoundedThree(Arc::new(BoundedSnapshot::new())),
        ),
        (
            (SnapshotName::Bounded, 4),
            BoundedFour(Arc::new(BoundedSnapshot::new())),
        ),
        (
            (SnapshotName::Bounded, 5),
            BoundedFive(Arc::new(BoundedSnapshot::new())),
        ),
        // LatticeSnapshot
        (
            (SnapshotName::Lattice, 2),
            LatticeTwo(Arc::new(LatticeSnapshot::new())),
        ),
        (
            (SnapshotName::Lattice, 3),
            LatticeThree(Arc::new(LatticeSnapshot::new())),
        ),
        (
            (SnapshotName::Lattice, 4),
            LatticeFour(Arc::new(LatticeSnapshot::new())),
        ),
        (
            (SnapshotName::Lattice, 5),
            LatticeFive(Arc::new(LatticeSnapshot::new())),
        ),
    ]);

    for n in MIN_NUM_THREADS..MAX_NUM_THREADS + 1 {
        group.bench_with_input(BenchmarkId::new("AAD+93/UnboundedAtomic", n), &n, |b, n| {
            b.iter(|| benchmark_snapshots(&snapshots, SnapshotName::UnboundedAtomic, *n))
        });
        group.bench_with_input(BenchmarkId::new("AAD+93/UnboundedMutex", n), &n, |b, n| {
            b.iter(|| benchmark_snapshots(&snapshots, SnapshotName::UnboundedMutex, *n))
        });
        group.bench_with_input(BenchmarkId::new("AAD+93/Bounded", n), &n, |b, n| {
            b.iter(|| benchmark_snapshots(&snapshots, SnapshotName::Bounded, *n))
        });
        group.bench_with_input(BenchmarkId::new("AR98/Lattice", n), &n, |b, n| {
            b.iter(|| benchmark_snapshots(&snapshots, SnapshotName::Lattice, *n))
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
