# todc

[![CI](https://github.com/kaymanb/todc/actions/workflows/ci.yml/badge.svg)](https://github.com/kaymanb/todc/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/kaymanb/todc/graph/badge.svg?token=BP1WOBRO9R)](https://codecov.io/gh/kaymanb/todc)

`todc` is a library of distributed computing algorithms, written in Rust.

## Overview

This is very experimental. The goal of this library is to bridge the gap between 
theory and practice by providing _usable_, _understandable_, and _correct_ 
implementations of algorithms from classic papers. 

### Message Passing

Unreleased, under active development.

For message passing systems, `todc-net` provides implementations for services 
that communicate over HTTP. 

### Shared Memory

For shared memory systems, `todc-mem` provides implementations for processes 
running on a single peice of hardware. 

#### Limitations

A lot of theoretical work in shared-memory distributed systems depends on the 
existence of unbounded, or at least sizable, 
[atomic](https://en.wikipedia.org/wiki/Atomic_semantics) memory. Even though
modern hardware does provide access to small amounts of 
[sequentially consistent](https://en.wikipedia.org/wiki/Sequential_consistency)
memory, usually at most 64 bits, this is rarely enough to meet the assumptions 
made in most papers. 

As such, while the algorithms in this crate have been written to be as correct
and usable as possible, they do lack some desired properties and may be of
little _practical_ value. 

#### Examples

Use a [snapshot object](https://en.wikipedia.org/wiki/Shared_snapshot_objects) to 
obtain a consistent view of progress being made by a set of threads.

```rs
use std::sync::Arc;
use std::thread;
use todc_mem::snapshot::{Snapshot, BoundedAtomicSnapshot};

const N: usize = 6;

let snapshot: Arc<BoundedAtomicSnapshot<N>> = Arc::new(BoundedAtomicSnapshot::new());

// Does some work, and returns what percent of the total
// amount of work has been completed.
fn do_work(i: usize) -> Option<u8> {
  // -- snipped --
}

// Each worker thread does some work and periodically updates
// its component of the snapshot with the amount of progress
// it has made so far.
let mut workers = Vec::new();
for i in 1..N {
    let mut snapshot = snapshot.clone();
    workers.push(thread::spawn(move || {
        while let Some(percent_complete) = do_work(i) {
            snapshot.update(i, percent_complete);
        }        
    }));
}

// The main thread waits until all workers have completed
// at least half of their work, before printing a message.
snapshot.update(0, 100);
loop {
    let view = snapshot.scan(0);
    if view.iter().all(|&p| p >= 50) {
        println!("We're at-least half-way done!");
        break;
    }
}
```

### Utilities

For general utilities, `todc-utils` provides some helpful implementations
for things such as specifying behaviour, and checking 
[linearizability](https://en.wikipedia.org/wiki/Linearizability).

#### Examples

Determine if a history of operations performed on some shared-object, like 
[`etcd`](https://etcd.io/), is actually linearizable. See `todc-utils/tests/etcd.rs`
for more details.

```rs
use todc_utils::linearizability::WGLChecker;
use todc_utils::specifications::etcd::{history_from_log, EtcdSpecification};

// Define a linearizability checker for an etcd (compare-and-swap) object.
type EtcdChecker = WGLChecker<EtcdSpecification>;

// Create a history of operations based on log output.
let history = history_from_log("todc-utils/tests/linearizability/etcd/etcd_001.log")

// Assert that the history of operations is actually linearizable.
assert!(EtcdChecker::is_linearizable(history));
```

## Development

### Code Coverage

Code coverage can be calculated with [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov). 
Some tests can only be run when certain features are enabled. To most-accurately
calculate code coverage, run:

```
cargo llvm-cov clean --workspace
cargo llvm-cov --no-report --workspace
cargo llvm-cov --no-report -p todc-mem --features shuttle --test snapshot
cargo llvm-cov --no-report -p todc-net --features turmoil --test abd_95
cargo llvm-cov report --open
```

