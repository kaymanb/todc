//! Implementations of atomic snapshot objects based on the paper by
//! Attiya and Rachman [[AR93]](https://doi.org/10.1137/S0097539795279463).
use super::Snapshot;

use crate::register::{AtomicRegister, Register};

#[derive(Clone, Copy, Default)]
struct Component<T: Copy + Default> {
    value: T,
    sequence: usize,
    counter: usize,
}

struct Classifier<T: Copy + Default, const N: usize> {
    registers: [AtomicRegister<Component<T>>; N],
}

impl<T: Copy + Default, const N: usize> Default for Classifier<T, N> {
    fn default() -> Self {
        Self {
            registers: [(); N].map(|_| AtomicRegister::<Component<T>>::new()),
        }
    }
}

/// An atomic snapshot object.
pub struct AtomicSnapshot<T: Copy + Default, const N: usize, const M: usize> {
    components: [AtomicRegister<Component<T>>; N],
    tree: CompleteBinaryTree<Classifier<T, N>>,
}

/// A complete binary tree.
#[derive(Debug)]
enum CompleteBinaryTree<T: Default> {
    Leaf(T),
    Node(T, Box<CompleteBinaryTree<T>>, Box<CompleteBinaryTree<T>>),
}

impl<T: Default> CompleteBinaryTree<T> {
    /// Creates a new complete binary tree of a given height.
    fn new(height: usize) -> Self {
        match height {
            1 => Self::Leaf(T::default()),
            _ => Self::Node(
                T::default(),
                Box::new(Self::new(height - 1)),
                Box::new(Self::new(height - 1)),
            ),
        }
    }

    fn level(&self) -> usize {
        match self {
            Self::Leaf(_) => 1,
            Self::Node(_, _, child) => child.level() + 1,
        }
    }
}

#[cfg(test)]
mod complete_binary_tree_tests {
    use super::CompleteBinaryTree;

    mod test_level {
        use super::*;

        #[test]
        fn test_leaf_has_level_one() {
            assert_eq!(CompleteBinaryTree::Leaf(1).level(), 1)
        }

        #[test]
        fn test_root_has_same_level_as_height() {
            assert_eq!(CompleteBinaryTree::<usize>::new(10).level(), 10);
        }

        #[test]
        fn test_child_has_one_fewer_level() {
            let root = CompleteBinaryTree::<usize>::new(3);
            let expected = root.level() - 1;
            if let CompleteBinaryTree::Node(_, left, right) = root {
                assert_eq!(left.level(), expected);
                assert_eq!(right.level(), expected);
            }
        }
    }
}
