//! Specifying the behavior of shared objects.
use std::fmt::Debug;
use std::hash::Hash;

pub mod etcd;
pub mod register;
pub mod snapshot;

/// A (sequential) specification of an object.
///
/// This trait defines how operations performed on the object affect its state.
///
/// # Examples
///
/// Consider the following specification for a register that stores a single
/// `u32` value. Initially, the register contains the value `0`.
///
/// ```
/// use todc_utils::Specification;
///
/// #[derive(Copy, Clone, Debug)]
/// enum RegisterOp {
///     Read(u32),
///     Write(u32),
/// }
///
/// use RegisterOp::{Read, Write};
///
/// struct RegisterSpec;
///
/// impl Specification for RegisterSpec {
///     type State = u32;
///     type Operation = RegisterOp;
///     
///     fn init(&self) -> Self::State {
///         0
///     }
///
///     fn apply(&self, operation: &Self::Operation, state: &Self::State) -> (bool, Self::State) {
///         match operation {
///             Read(value) => (value == state, *state),
///             Write(value) => (true, *value),
///         }
///     }
/// }
/// ```
///
/// A `Write` operation is always valid, as is a `Read` operation that returns
/// the value of the most-recent write.
///
/// ```
/// # use todc_utils::Specification;
/// # #[derive(Copy, Clone, Debug)]
/// # enum RegisterOp {
/// #     Read(u32),
/// #     Write(u32),
/// # }
/// # use RegisterOp::{Read, Write};
/// # struct RegisterSpec;
/// # impl Specification for RegisterSpec {
/// #     type State = u32;
/// #     type Operation = RegisterOp;
/// #     
/// #     fn init(&self) -> Self::State {
/// #         0
/// #     }
/// #     fn apply(&self, operation: &Self::Operation, state: &Self::State) -> (bool, Self::State) {
/// #         match operation {
/// #             Read(value) => (value == state, *state),
/// #             Write(value) => (true, *value),
/// #         }
/// #     }
/// # }
/// let spec = RegisterSpec {};
/// let (is_valid, new_state) = spec.apply(&Write(1), &spec.init());
/// assert!(is_valid);
/// assert_eq!(new_state, 1);
///
/// let (is_valid, new_state) = spec.apply(&Read(1), &new_state);
/// assert!(is_valid);
/// assert_eq!(new_state, 1);
/// ```
///
/// On the other hand, a `Read` operation that returns a different value to
/// the one currently stored in the register is **not** valid.
///
/// ```
/// # use todc_utils::Specification;
/// # #[derive(Copy, Clone, Debug)]
/// # enum RegisterOp {
/// #     Read(u32),
/// #     Write(u32),
/// # }
/// # use RegisterOp::{Read, Write};
/// # struct RegisterSpec;
/// # impl Specification for RegisterSpec {
/// #     type State = u32;
/// #     type Operation = RegisterOp;
/// #     
/// #     fn init(&self) -> Self::State {
/// #         0
/// #     }
/// #     fn apply(&self, operation: &Self::Operation, state: &Self::State) -> (bool, Self::State) {
/// #         match operation {
/// #             Read(value) => (value == state, *state),
/// #             Write(value) => (true, *value),
/// #         }
/// #     }
/// # }
/// let spec = RegisterSpec {};
/// let (_, new_state) = spec.apply(&Write(1), &spec.init());
/// let (is_valid, _) = spec.apply(&Read(42), &new_state);
/// assert!(!is_valid);
/// ```

pub trait Specification {
    type State: Clone + Eq + Hash + Debug;
    type Operation: Clone + Debug;

    /// Returns an initial state for the object.
    fn init(&self) -> Self::State;

    /// Returns whether applying an operation to a given state is valid, and
    /// the new state that occurs after the operation has been applied.
    ///
    /// If the operation is not valid, then the state of the object should not change.
    fn apply(&self, op: &Self::Operation, state: &Self::State) -> (bool, Self::State);
}
