//! Implementations of atomic snapshot objects based on the paper by
//! Attiya and Rachman [[AR93]](https://doi.org/10.1137/S0097539795279463).
use core::array::from_fn;

use super::Snapshot;

use crate::register::{AtomicRegister, Register};

/// The contents of one component of a snapshot object.
#[derive(Clone, Copy, Default)]
struct Component<T: Copy + Default> {
    value: T,
    sequence: usize,
    counter: usize,
}

/// A view of all components of a snapshot object.
#[derive(Clone, Copy)]
struct View<T: Copy + Default, const N: usize> {
    components: [Component<T>; N],
}

impl<T: Copy + Default, const N: usize> View<T, N> {
    /// Returns the union of an array of views.
    ///
    /// The union of an array of views V_1...V_N is a new view V where each
    /// component V[i] is equal to the component V_j[i] with maximal _sequence_ field.
    fn union_many(views: [View<T, N>; N]) -> View<T, N> {
        // TODO: Implement this!
    }
}

impl<T: Copy + Default, const N: usize> Default for View<T, N> {
    fn default() -> Self {
        Self {
            components: [(); N].map(|_| Component::default()),
        }
    }
}

/// An object for classifying processes into two disjoint groups.
struct Classifier<T: Copy + Default, const N: usize> {
    registers: [AtomicRegister<View<T, N>>; N],
}

impl<T: Copy + Default, const N: usize> Default for Classifier<T, N> {
    fn default() -> Self {
        Self {
            registers: [(); N].map(|_| AtomicRegister::new()),
        }
    }
}

impl<T: Copy + Default, const N: usize> Classifier<T, N> {
    fn collect(&self) -> [View<T, N>; N] {
        from_fn(|i| self.registers[i].read())
    }

    /// TODO: Explain this.
    fn classify(&self, i: usize, knowledge_bound: usize, view: View<T, N>) -> View<T, N> {
        self.registers[i].write(view);
        knowledge = View::union_many(self.collect());
        // TODO: Continue here maybe.
    }
}

/// An N-process M-shot atomic snapshot object.
pub struct AtomicSnapshot<T: Copy + Default, const N: usize, const M: usize> {
    components: [AtomicRegister<Component<T>>; N],
    root: CompleteBinaryTree<Classifier<T, N>>,
}

impl<T: Copy + Default, const N: usize, const M: usize> AtomicSnapshot<T, N, M> {
    /// Returns a view of the snapshot object, and updates the ith component to
    /// contain the input value.
    fn scate(&self, i: usize, value: T) -> [T; N] {}
}

impl<T: Copy + Default, const N: usize, const M: usize> Snapshot<N> for AtomicSnapshot<T, N, M> {
    type Value = T;

    fn new() -> Self {
        Self {
            components: [(); N].map(|_| AtomicRegister::new()),
            root: CompleteBinaryTree::new(M),
        }
    }

    fn scan(&self, i: usize) -> [Self::Value; N] {
        self.scate(i, self.components[i].read().value)
    }

    fn update(&self, i: usize, value: Self::Value) -> () {
        self.scate(i, value);
    }
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

    /// Returns the level of the node inside the tree.
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
