use std::sync::Arc;
use std::thread;

use todc::snapshot::{ar_98::AtomicSnapshot, Snapshot};

fn main() {
    const SIZE: usize = 10;
    const M: u32 = 8;
    let snapshot: Arc<AtomicSnapshot<usize, SIZE, M>> = Arc::new(AtomicSnapshot::new());

    for i in 0..SIZE {
        let snapshot = Arc::clone(&snapshot);
        thread::spawn(move || {
            snapshot.update(i, i + 1);
        });
    }

    println!("Read {:?} from snapshot", snapshot.scan(0));
}
