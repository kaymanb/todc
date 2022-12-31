use todc::register::{AtomicRegister};

#[derive(Debug)]
struct BinaryTree<T: Default> {
    value: T,
    left: Option<Box<BinaryTree<T>>>,
    right: Option<Box<BinaryTree<T>>>
}

impl<T: Default> BinaryTree<T> {

    fn new(value: T) -> Self {
        BinaryTree {
            value: value,
            left: None,
            right: None
        } 
    }

    fn complete(depth: usize) -> Self {
        match depth {
            1 => Self::new(T::default()),
            _ => {
                let mut root = Self::new(T::default());
                root.left = Some(Box::new(Self::complete(depth - 1)));
                root.right = Some(Box::new(Self::complete(depth - 1)));
                root
            }
        } 
    }
}

fn main() {
    let bt: BinaryTree<AtomicRegister<usize>> = BinaryTree::complete(3);
    print!("{:?}", bt)
}
