use std::sync::Arc;
use std::thread;

use todc::snapshot::{Snapshot, UnboundedAtomicSnapshot};

fn main() {
    const SIZE: usize = 10;
    let snapshot: Arc<UnboundedAtomicSnapshot<usize, SIZE>> = Arc::new(UnboundedAtomicSnapshot::new(0));

    for i in 0..SIZE {
        let snapshot = Arc::clone(&snapshot);
        thread::spawn(move || {
            snapshot.update(i, i);
        });
    }

    println!("Read {:?} from snapshot", snapshot.scan());
}
