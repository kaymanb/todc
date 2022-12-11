use std::sync::Arc;
use std::thread;

use todc::snapshot::{aad_plus::BoundedAtomicSnapshot, Snapshot};

fn main() {
    const SIZE: usize = 10;
    let snapshot: Arc<BoundedAtomicSnapshot<usize, SIZE>> = Arc::new(BoundedAtomicSnapshot::new(0));

    for i in 0..SIZE {
        let snapshot = Arc::clone(&snapshot);
        thread::spawn(move || {
            snapshot.update(i, i + 1);
        });
    }

    println!("Read {:?} from snapshot", snapshot.scan(0));
}
