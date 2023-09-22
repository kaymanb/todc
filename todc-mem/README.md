# todc-mem

Algorithms for shared-memory distributed systems.

## Limitations

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

## Examples

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

## Development

Some tests make use of [shuttle](https://github.com/awslabs/shuttle) for 
_randomized concurrency testing_. To run tests that require this feature, do:
```
cargo test --features shuttle --test MODULE --release
```
