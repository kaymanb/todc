//! Checking [linearizability](https://en.wikipedia.org/wiki/Linearizability).  
//!
//! See _Testing for Linearizability_ by Gavin Lowe [\[L16\]](https://doi.org/10.1002/cpe.3928) and
//! _Faster Linearizability Checking via P-Compositionality_ by Horn and Kroening
//! [\[HK15\]](https://arxiv.org/abs/1504.00204).
//!
//! See [`linearizability-checker``](https://github.com/ahorn/linearizability-checker) for C++
//! implementations of the algorithms contained in this module. For implementations in Go,
//! see [`porcupine`](https://github.com/anishathalye/porcupine).

use crate::linearizability::history::{Entry, History};
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;

pub mod history;

pub trait Specification {
    type State: Clone + Eq + Hash + Debug;
    type Operation: Clone + Debug;

    /// Returns an initial state for the specification.
    fn init(&self) -> Self::State;

    /// Returns the state that results from a valid call and response pair.
    fn apply(&self, op: &Self::Operation, state: &Self::State) -> (bool, Self::State);
}

pub struct WLGChecker {}

type OperationEntry<S> = Entry<<S as Specification>::Operation>;
type OperationCall<S> = (
    (OperationEntry<S>, OperationEntry<S>),
    <S as Specification>::State,
);

impl WLGChecker {
    pub fn is_linearizable<S: Specification>(spec: S, mut history: History<S::Operation>) -> bool {
        let mut state = spec.init();
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
                        let (is_valid, new_state) = spec.apply(&response.operation, &state);
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

        fn init(&self) -> Self::State {
            0
        }

        fn apply(&self, operation: &Self::Operation, state: &Self::State) -> (bool, Self::State) {
            match operation {
                Read(value) => (value == state, *state),
                Write(value) => (true, *value),
            }
        }
    }

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
            assert!(WLGChecker::is_linearizable(IntegerRegisterSpec, history));
        }

        #[test]
        fn rejects_invalid_reads() {
            let history = History::from_actions(vec![
                (0, Call(Write(1))),
                (0, Response(Write(1))),
                (0, Call(Read(2))),
                (0, Response(Read(2))),
            ]);
            assert!(!WLGChecker::is_linearizable(IntegerRegisterSpec, history));
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
            assert!(WLGChecker::is_linearizable(IntegerRegisterSpec, history));
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
            assert!(!WLGChecker::is_linearizable(IntegerRegisterSpec, history));
        }
    }
}
