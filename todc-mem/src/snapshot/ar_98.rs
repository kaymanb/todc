//! Implementations of atomic snapshot objects based on the paper by
//! Attiya and Rachman [\[AR93\]](https://doi.org/10.1137/S0097539795279463).
use super::Snapshot;
use crate::register::{MutexRegister, Register};
use core::array::from_fn;

/// The contents of one component of a snapshot object.
#[derive(Clone, Copy, Default)]
struct Component<T: Copy + Default> {
    value: T,
    sequence: u32,
    counter: u32,
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
        View {
            components: from_fn(|i| {
                let max = views.iter().max_by_key(|v| v.components[i].sequence);
                max.unwrap().components[i]
            }),
        }
    }

    /// Returns the size of the view, which is the number of operations that were
    /// performed on the snapshot object before the view was obtained.
    ///
    /// Intutively, the size of the view corresponds to the amount of knowledge
    /// that the view contains.
    fn size(&self) -> u32 {
        self.components.map(|c| c.counter).iter().sum()
    }

    /// Returns an array of values stored in the components of the view.
    fn values(&self) -> [T; N] {
        self.components.map(|c| c.value)
    }
}

impl<T: Copy + Default, const N: usize> Default for View<T, N> {
    fn default() -> Self {
        Self {
            components: [(); N].map(|_| Component::default()),
        }
    }
}

/// Groups for classifying processes based on their view of the components
/// of a snapshot object.
enum Group<T: Copy + Default, const N: usize> {
    Primary(View<T, N>),
    Secondary,
}

/// An object for classifying processes into two disjoint groups and updating
/// their knowledge of the contents of a snapshot objects components.
struct Classifier<T: Copy + Default, const N: usize> {
    registers: [MutexRegister<View<T, N>>; N],
}

impl<T: Copy + Default, const N: usize> Default for Classifier<T, N> {
    fn default() -> Self {
        Self {
            registers: [(); N].map(|_| MutexRegister::new()),
        }
    }
}

impl<T: Copy + Default, const N: usize> Classifier<T, N> {
    /// Reads from each register and returns an array of the results.
    fn collect(&self) -> [View<T, N>; N] {
        from_fn(|i| self.registers[i].read())
    }

    /// Classify the input process into either a _primary_ or _secondary group_, and
    /// update the knowledge the process has about contents of the snapshot object.
    ///
    /// Calling processes are classified into disjoint groups. Processes in the primary
    /// group may learn additional information about the contents of the snapshot
    /// object, and recieve an updated view in response. Processes in the secondary
    /// group retain their original knowledge.
    ///
    /// The input parameter _knowledge_bound_ is used to determine whether a process
    /// belongs to the primary or secondary group. Intuitively, if the amount of
    /// information a process knows about the contents of the snapshot object is
    /// greater than the knowledge bound, then it is placed in the primary group.
    /// If the amount of knowledge a process has is less than the knowledge bound,
    /// it is placed in the secondary group.
    fn classify(&self, i: usize, knowledge_bound: u32, view: View<T, N>) -> Group<T, N> {
        self.registers[i].write(view);
        let union = View::union_many(self.collect());
        if union.size() > knowledge_bound {
            Group::Primary(union)
        } else {
            Group::Secondary
        }
    }
}

/// An N-process M-shot mutex-based snapshot object.
// TODO: Modify this implementation to an infinity-shot snapshot object, as
// described in the paper.
pub struct LatticeMutexSnapshot<T: Copy + Default, const N: usize, const M: u32> {
    components: [MutexRegister<Component<T>>; N],
    root: Box<CompleteBinaryTree<Classifier<T, N>>>,
}

impl<T: Copy + Default, const N: usize, const M: u32> LatticeMutexSnapshot<T, N, M> {
    /// Reads from each register and returns an array of the results.
    fn collect(&self) -> View<T, N> {
        View {
            components: from_fn(|i| self.components[i].read()),
        }
    }

    /// Returns an array of values based on the contents of the snapshot object.
    ///
    /// The values are determined by having the process traverse through log_2(M)
    /// levels of a complete binary tree. At each level, the knowledge the process
    /// has about the contents of the snapshot object either increases (and the
    /// process decends to the right) or stays the same (and the process decends to
    /// the left). Once the process reaches a leaf, it returns an array of values
    /// based on the knowledge it obtained during this traversal.
    fn traverse(
        i: usize,
        node: &CompleteBinaryTree<Classifier<T, N>>,
        view: View<T, N>,
        label: u32,
    ) -> [T; N] {
        match node {
            CompleteBinaryTree::Leaf(cls) => match cls.classify(i, label, view) {
                Group::Primary(union) => union.values(),
                Group::Secondary => view.values(),
            },
            CompleteBinaryTree::Node(cls, left, right) => match cls.classify(i, label, view) {
                Group::Primary(union) => {
                    let label = label + (M / 2_u32.pow(right.level() + 1));
                    Self::traverse(i, right, union, label)
                }
                Group::Secondary => {
                    let label = label - (M / 2_u32.pow(left.level() + 1));
                    Self::traverse(i, left, view, label)
                }
            },
        }
    }

    /// Returns a view of the snapshot object and updates the ith component to
    /// contain the input value.
    fn scate(&self, i: usize, value: T) -> [T; N] {
        let component = self.components[i].read();
        self.components[i].write(Component {
            value,
            counter: component.counter + 1,
            sequence: component.sequence + 1,
        });
        Self::traverse(i, &self.root, self.collect(), M)
    }
}

impl<T: Copy + Default, const N: usize, const M: u32> Snapshot<N>
    for LatticeMutexSnapshot<T, N, M>
{
    type Value = T;

    /// Create a new snapshot object.
    ///
    /// # Panics
    ///
    /// This method will panic if M, the number of operations that can be
    /// applied to the object, is not a power of 2.
    fn new() -> Self {
        // log_2(M) must be an integer to construct a complete binary tree of
        // that height.
        if !((M as f32).log2() == (M as f32).log2().floor()) {
            panic!("The number M of supported operations must be a power of 2")
        }
        let height = (M as f32).log2().floor() as u32;
        Self {
            components: [(); N].map(|_| MutexRegister::new()),
            root: Box::new(CompleteBinaryTree::new(height)),
        }
    }

    fn scan(&self, i: usize) -> [Self::Value; N] {
        self.scate(i, self.components[i].read().value)
    }

    fn update(&self, i: usize, value: Self::Value) {
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
    fn new(height: u32) -> Self {
        match height {
            1 => Self::Leaf(T::default()),
            _ => Self::Node(
                T::default(),
                Box::new(Self::new(height - 1)),
                Box::new(Self::new(height - 1)),
            ),
        }
    }

    /// Returns the _level_ of the node inside the tree.
    ///
    /// The level of a node is the height of the tree rooted
    /// at that node.
    // TODO: This recursive implementation is slow... Should memoize this.
    fn level(&self) -> u32 {
        match self {
            Self::Leaf(_) => 1,
            Self::Node(_, _, child) => child.level() + 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LatticeMutexSnapshot, Snapshot};

    #[test]
    fn reads_and_writes() {
        let snapshot: LatticeMutexSnapshot<usize, 3, 16> = LatticeMutexSnapshot::new();
        assert_eq!([0, 0, 0], snapshot.scan(0));
        snapshot.update(1, 1);
        snapshot.update(2, 2);
        assert_eq!([0, 1, 2], snapshot.scan(0));
        snapshot.update(0, 10);
        snapshot.update(1, 11);
        snapshot.update(2, 12);
        assert_eq!([10, 11, 12], snapshot.scan(0));
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
