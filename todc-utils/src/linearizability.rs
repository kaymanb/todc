//! Checking [linearizability](https://en.wikipedia.org/wiki/Linearizability) of a
//! history of operations applied to a shared object.
//!
//! For more information, see the documentation of the [`WGLChecker`] and [`History`] structs.
use std::collections::HashSet;
use std::marker::PhantomData;

use crate::linearizability::history::{Entry, History};
use crate::specifications::Specification;

pub mod history;

/// A linearizability checker.
///
/// An implementation of the algorithm originally defined by Jeannette Wing and Chun Gong
/// [\[WG93\]](https://www.cs.cmu.edu/~wing/publications/WingGong93.pdf), and
/// extended by Gavin Lowe [\[L17\]](http://www.cs.ox.ac.uk/people/gavin.lowe/LinearizabiltyTesting/).
/// This particular implementation is based on the description given by Alex Horn
/// and Daniel Kroenig [\[HK15\]](https://arxiv.org/abs/1504.00204).
///
/// Given a history of operations, the algorithm works by linearizing each operation
/// as soon as possible. When an operation cannot be linearized, it backtracks and
/// proceeds with the next operation. Memoization occurs by caching each partial
/// linearization, and preventing the algorithm from continuing its search when it
/// is already known that the state of the object and remaining operations have no
/// valid linearization.
///
///
/// # Examples
///
/// Consider the following [`Specification`] of a register containing `u32` values.
///
/// ```
/// use todc_utils::specifications::Specification;
///
/// #[derive(Copy, Clone, Debug)]
/// enum RegisterOp {
///     Read(Option<u32>),
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
///     fn init() -> Self::State {
///         0
///     }
///
///     fn apply(operation: &Self::Operation, state: &Self::State) -> (bool, Self::State) {
///         match operation {
///             // A read is valid if the value returned is equal to the
///             // current state. Reads always leave the state unchanged.
///             Read(value) => match value {
///                 Some(value) => (value == state, *state),
///                 None => (false, *state)
///             },
///             // Writes are always valid, and update the state to be
///             // equal to the value being written.
///             Write(value) => (true, *value),
///         }
///     }
/// }
/// ```    
///
/// Using the [`Action::Call`](history::Action::Call) and
/// [`Action::Response`](history::Action::Response) types, we can model read
/// and write operations as follows:
/// * The call of a read operation is modeled by `Call(Read(None))` and a
///   response containing the value `x` is modeled by `Response(Read(Some(x)))`.
///   We are required to use an [`Option`] because the value being read cannot
///   be known until the register responds.
/// * Similarily, the call of a write operation with the value `y` is modeled
///   by `Call(Write(y))` and the response is modeled by `Response(Write(y))`.
///
/// Next, we can define a linearizability for this specification, and check some
/// histories.
/// ```
/// # use todc_utils::specifications::Specification;
/// # #[derive(Copy, Clone, Debug)]
/// # enum RegisterOp {
/// #     Read(Option<u32>),
/// #     Write(u32),
/// # }
/// # use RegisterOp::{Read, Write};
/// # struct RegisterSpec;
/// # impl Specification for RegisterSpec {
/// #     type State = u32;
/// #     type Operation = RegisterOp;
/// #     fn init() -> Self::State {
/// #         0
/// #     }
/// #     fn apply(operation: &Self::Operation, state: &Self::State) -> (bool, Self::State) {
/// #         match operation {
/// #             // A read is valid if the value returned is equal to the
/// #             // current state. Reads always leave the state unchanged.
/// #             Read(value) => match value {
/// #                 Some(value) => (value == state, *state),
/// #                 None => (false, *state)
/// #             },
/// #             // Writes are always valid, and update the state to be
/// #             // equal to the value being written.
/// #             Write(value) => (true, *value),
/// #         }
/// #     }
/// # }
/// use todc_utils::linearizability::{WGLChecker, history::{History, Action::{Call, Response}}};
///
/// type RegisterChecker = WGLChecker<RegisterSpec>;
///
/// // A history of sequantial operations is always linearizable.
/// // PO |------|          Write(0)
/// // P1          |------| Read(Some(0))
/// let history = History::from_actions(vec![
///     (0, Call(Write(0))),
///     (0, Response(Write(0))),
///     (1, Call(Read(None))),
///     (1, Response(Read(Some(0)))),
/// ]);
/// assert!(RegisterChecker::is_linearizable(history));
///
/// // Concurrent operations might not be linearized
/// // in the order in which they are called.
/// // PO |--------------|  Write(0)
/// // P1  |--------------| Write(1)
/// // P2    |---|          Read(Some(1))
/// // P3           |---|   Read(Some(0))
/// let history = History::from_actions(vec![
///     (0, Call(Write(0))),
///     (1, Call(Write(1))),
///     (2, Call(Read(None))),
///     (2, Response(Read(Some(1)))),
///     (3, Call(Read(None))),
///     (3, Response(Read(Some(0)))),
///     (0, Response(Write(0))),
///     (1, Response(Write(1))),
/// ]);
/// assert!(RegisterChecker::is_linearizable(history));
///
/// // A sequentially consistent history is **not**
/// // necessarily linearizable.
/// // PO |---|             Write(0)
/// // P1 |---|             Write(1)
/// // P2       |---|       Read(Some(1))
/// // P3             |---| Read(Some(0))
/// let history = History::from_actions(vec![
///     (0, Call(Write(0))),
///     (1, Call(Write(1))),
///     (0, Response(Write(0))),
///     (1, Response(Write(1))),
///     (2, Call(Read(None))),
///     (2, Response(Read(Some(1)))),
///     (3, Call(Read(None))),
///     (3, Response(Read(Some(0)))),
/// ]);
/// assert!(!RegisterChecker::is_linearizable(history));
/// ```    
///   
/// For examples of using [`WGLChecker`] to check the linearizability of more
/// complex histories, see
/// [`todc-mem/tests/snapshot/common.rs`](https://github.com/kaymanb/todc/blob/main/todc-mem/tests/snapshot/common.rs)
/// or
/// [`todc-utils/tests/linearizability/etcd.rs`](https://github.com/kaymanb/todc/blob/main/todc-utils/tests/linearizability/etcd.rs).
///
/// # Implementations in Other Languages
///
/// For an implementation in C++, see [`linearizability-checker`](https://github.com/ahorn/linearizability-checker).
/// For an implementation in Go, see [`porcupine`](https://github.com/anishathalye/porcupine).
pub struct WGLChecker<S: Specification> {
    data_type: PhantomData<S>,
}

type OperationEntry<S> = Entry<<S as Specification>::Operation>;
type OperationCall<S> = (
    (OperationEntry<S>, OperationEntry<S>),
    <S as Specification>::State,
);

impl<S: Specification> WGLChecker<S> {
    /// Returns whether the history of operations is linearizable with respect to the specification.
    pub fn is_linearizable(mut history: History<S::Operation>) -> bool {
        let mut state = S::init();
        let mut linearized = vec![false; history.len()];
        let mut calls: Vec<OperationCall<S>> = Vec::new();
        let mut cache: HashSet<(Vec<bool>, S::State)> = HashSet::new();
        let mut curr = 0;
        loop {
            if history.is_empty() {
                return true;
            }
            match &history[curr] {
                Entry::Call(call) => match &history[history.index_of_id(call.response)] {
                    Entry::Call(_) => panic!("Response cannot be a call entry"),
                    Entry::Response(response) => {
                        let (is_valid, new_state) = S::apply(&response.operation, &state);
                        let mut changed = false;
                        if is_valid {
                            let mut tmp_linearized = linearized.clone();
                            tmp_linearized[call.id] = true;
                            changed = cache.insert((tmp_linearized, new_state.clone()));
                        }
                        if changed {
                            linearized[call.id] = true;
                            let call = history.lift(curr);
                            calls.push((call, state));
                            state = new_state;
                            curr = 0;
                        } else {
                            curr += 1;
                        }
                    }
                },
                Entry::Response(_) => match calls.pop() {
                    None => return false,
                    Some(((call, response), old_state)) => {
                        state = old_state;
                        linearized[call.id()] = false;
                        let (call_index, _) = history.unlift(call, response);
                        curr = call_index + 1;
                    }
                },
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use history::Action::*;

    #[derive(Copy, Clone, Debug)]
    enum RegisterOperation {
        Read(u32),
        Write(u32),
    }

    use RegisterOperation::*;

    struct IntegerRegisterSpec;

    impl Specification for IntegerRegisterSpec {
        type State = u32;
        type Operation = RegisterOperation;

        fn init() -> Self::State {
            0
        }

        fn apply(operation: &Self::Operation, state: &Self::State) -> (bool, Self::State) {
            match operation {
                Read(value) => (value == state, *state),
                Write(value) => (true, *value),
            }
        }
    }

    type RegisterChecker = WGLChecker<IntegerRegisterSpec>;

    mod is_linearizable {
        use super::*;

        #[test]
        fn accepts_sequential_read_and_write() {
            let history = History::from_actions(vec![
                (0, Call(Write(1))),
                (0, Response(Write(1))),
                (0, Call(Read(1))),
                (0, Response(Read(1))),
            ]);
            assert!(RegisterChecker::is_linearizable(history));
        }

        #[test]
        fn rejects_invalid_reads() {
            let history = History::from_actions(vec![
                (0, Call(Write(1))),
                (0, Response(Write(1))),
                (0, Call(Read(2))),
                (0, Response(Read(2))),
            ]);
            assert!(!RegisterChecker::is_linearizable(history));
        }

        #[test]
        fn accepts_writes_in_reverse_order() {
            // Accepts the following history, in which processes
            // P1, P2, and P3 must linearize their writes in the
            // reverse order in which they are called.
            // P0 |--------------------| Write(1)
            // P1 |--------------------| Write(2)
            // P2 |--------------------| Write(3)
            // P3   |--|                 Read(3)
            // P3          |--|          Read(2)
            // P3                 |--|   Read(1)
            let history = History::from_actions(vec![
                (0, Call(Write(1))),
                (1, Call(Write(2))),
                (2, Call(Write(3))),
                (3, Call(Read(3))),
                (3, Response(Read(3))),
                (3, Call(Read(2))),
                (3, Response(Read(2))),
                (3, Call(Read(1))),
                (3, Response(Read(1))),
                (0, Response(Write(1))),
                (1, Response(Write(2))),
                (2, Response(Write(3))),
            ]);
            assert!(RegisterChecker::is_linearizable(history));
        }

        #[test]
        fn rejects_sequentially_consistent_reads() {
            // Rejects the following history, in which P1 and P2 read
            // different values while overlapping with P0s write. Notice
            // that this history is _sequentially consistent_, as P2s
            // read could be re-ordered to complete prior to any other
            // operation.
            // P0 |-------------------| Write(1)
            // P1      |--|             Read(1)
            // P2              |--|     Read(0)
            let history = History::from_actions(vec![
                (0, Call(Write(1))),
                (1, Call(Read(1))),
                (1, Response(Read(1))),
                (2, Call(Read(0))),
                (2, Response(Read(0))),
                (0, Response(Write(1))),
            ]);
            assert!(!RegisterChecker::is_linearizable(history));
        }
    }
}
