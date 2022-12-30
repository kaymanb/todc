// use std::sync::Arc;
// use std::thread;

// use todc::snapshot::{aad_plus_93::BoundedAtomicSnapshot, Snapshot};

// fn main() {
//     const SIZE: usize = 10;
//     let snapshot: Arc<BoundedAtomicSnapshot<usize, SIZE>> = Arc::new(BoundedAtomicSnapshot::new(0));

//     for i in 0..SIZE {
//         let snapshot = Arc::clone(&snapshot);
//         thread::spawn(move || {
//             snapshot.update(i, i + 1);
//         });
//     }

//     println!("Read {:?} from snapshot", snapshot.scan(0));
// }

struct Node<T> {
    value: T,
    left: Option<Box<Node<T>>>,
    right: Option<Box<Node<T>>>
}

fn main() {
    let bst = Node {
        value: 0,
        left: Some(Box::new(Node { value: 1, left: None, right: None })),
        right: None
    };
    print!("{}", bst.value)
}
