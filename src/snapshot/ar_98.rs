//! Implementations of atomic snapshot objects based on the paper by
//! Attiya and Rachman [[AR93]](https://doi.org/10.1137/S0097539795279463).
use core::array::from_fn;
use num_traits;
use super::Snapshot;
use crate::register::{AtomicRegister, Register};
use ProcessGroup::*;
use CompleteBinaryTree::{Leaf, Node};

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

    /// Return the size of the view, which is the number of operations that were
    /// performed on the snapshot object before the view was obtained.
    ///
    /// Intutively, the size of the view corresponds to the amount of knowledge
    /// that the view contains.
    fn size(&self) -> u32 {
        self.components.map(|c| c.counter).iter().sum()
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
enum ProcessGroup<T: Copy + Default, const N: usize> {
    Primary(View<T, N>),
    Secondary(View<T, N>)
}

/// An object for classifying processes into two disjoint groups and updating
/// their knowledge of the contents of a snapshot objects components.
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

    /// Returns a view, containing information about the contents of a snapshot objects
    /// components.
    ///
    /// Calling processes are classified into two disjoint groups, and the view that
    /// is returned depends on which group the process belongs to. Processes in the
    /// _secondary_ group retain their original knowledge, and receive the same view
    /// they used as input. Processes in the _primary_ group may learn additional
    /// information about the contents of the snapshot object, and recieve an updated
    /// view.
    ///
    /// The input parameter _knowledge_bound_ is used to determine whether a process
    /// belongs to the primary or secondary group. Intuitively, if the amount of
    /// information a process knows about the contents of the snapshot object is
    /// greater than the knowledge bound, then it is placed in the primary group.
    /// If the amount of knowledge a process has is less than the knowledge bound,
    /// it is placed in the secondary group.
    fn classify(&self, i: usize, knowledge_bound: u32, view: View<T, N>) -> ProcessGroup<T, N> {
        self.registers[i].write(view);
        let union = View::union_many(self.collect());
        if union.size() > knowledge_bound {
            Primary(union)
        } else {
            Secondary(view)
        }
    }
}

/// An N-process M-shot atomic snapshot object.
pub struct AtomicSnapshot<T: Copy + Default, const N: usize, const M: u32> {
    components: [AtomicRegister<Component<T>>; N],
    root: Box<CompleteBinaryTree<Classifier<T, N>>>,
}

impl<T: Copy + Default, const N: usize, const M: u32> AtomicSnapshot<T, N, M> {

    fn left_label(label: u32, level: u32) -> u32 {
        label - (M/2_u32.pow(level + 1))
    }

    fn right_label(label: u32, level: u32) -> u32 {
        label + (M/2_u32.pow(level + 1))
    }

    fn collect(&self) -> View<T, N> {
        View {
            components: from_fn(|i| self.components[i].read())
        }
    }
    
    fn traverse(i: usize, node: Box<CompleteBinaryTree<Classifier<T, N>>>, view: View<T, N>, label: u32) -> [T; N] {
        match *node {
            Leaf(cls) => {
                match cls.classify(i, label, view) {
                    Primary(union) => return union.components.map(|c| c.value),
                    Secondary(_) => return view.components.map(|c| c.value),
                }
            },
            Node(cls, left, right) => {
                match cls.classify(i, label, view) {
                    Primary(union) => {
                        let label = Self::right_label(label, right.level());
                        Self::traverse(i, right, union, label)
                    },
                    Secondary(_) => {
                        let label = Self::left_label(label, left.level());
                        Self::traverse(i, left, view, label)
                    }
                }
            }
        } 
    }

    /// Returns a view of the snapshot object and updates the ith component to
    /// contain the input value.
    fn scate(&self, i: usize, value: T) -> [T; N] {
        let component = self.components[i].read();
        self.components[i].write(Component {
            value,
            counter: component.counter + 1,
            sequence: component.sequence + 1
        });
        Self::traverse(i, self.root, self.collect(), M)
    }
}

impl<T: Copy + Default, const N: usize, const M: u32> Snapshot<N> for AtomicSnapshot<T, N, M> {
    type Value = T;

    fn new() -> Self {
        // The height of the complete binary tree should be log_2(M), but
        // computing logs of generic integers requires type-casts.
        let num_shots: f32 = num_traits::cast(M).unwrap();
        let height: u32 = num_traits::cast(num_shots.log2()).unwrap();
        Self {
            components: [(); N].map(|_| AtomicRegister::new()),
            root: Box::new(CompleteBinaryTree::new(height)),
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

    /// Returns the level of the node inside the tree.
    // TODO: Memoize this.
    fn level(&self) -> u32 {
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
