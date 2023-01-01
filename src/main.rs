use todc::register::AtomicRegister;

#[derive(Debug)]
enum CompleteBinaryTree<T: Default> {
    Leaf(T),
    Node(T, Box<CompleteBinaryTree<T>>, Box<CompleteBinaryTree<T>>),
}

impl<T: Default> CompleteBinaryTree<T> {
    fn new(depth: usize) -> Self {
        match depth {
            1 => Self::Leaf(T::default()),
            _ => Self::Node(
                T::default(),
                Box::new(Self::new(depth - 1)),
                Box::new(Self::new(depth - 1)),
            ),
        }
    }
}

fn main() {
    let bt: CompleteBinaryTree<AtomicRegister<usize>> = CompleteBinaryTree::new(2);
    print!("{:?}", bt)
}
