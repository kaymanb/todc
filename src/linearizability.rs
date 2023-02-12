//! Checking [linearizability](https://en.wikipedia.org/wiki/Linearizability).  
//!
//! See _Testing for Linearizability_ by Gavin Lowe [\[L16\]](https://doi.org/10.1002/cpe.3928) and
//! _Faster Linearizability Checking via P-Compositionality_ by Horn and Kroening
//! [\[HK15\]](https://arxiv.org/abs/1504.00204). For a Go implementation, see [Porcupine](https://github.com/anishathalye/porcupine).

use crate::linearizability::history::{Action, Entry, History};
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;

pub mod history;

pub trait Specification {
    type State: Clone + Eq + Hash + Debug;
    type Operation: Clone + Debug;

    /// Returns an initial state for the specification.
    fn init(&self) -> Self::State;

    /// Returns the result of applying the operation.
    // TODO: Maybe this shouldn't take Action::Call?
    fn apply(&self, action: Action<Self::Operation>, state: Self::State) -> (bool, Self::State);
}

pub struct WLGChecker<S: Specification> {
    pub spec: S,
}

impl<S: Specification> WLGChecker<S> {
    pub fn is_linearizable(&self, mut history: History<S::Operation>) -> bool {
        let mut state = self.spec.init();
        let mut linearized = vec![false; history.len()];
        let mut calls: Vec<((Entry<S::Operation>, Entry<S::Operation>), S::State)> = Vec::new();
        let mut cache: HashSet<(Vec<bool>, S::State)> = HashSet::new();
        let mut curr = 0;
        loop {
            if history.len() == 0 {
                return true;
            }
            if history[curr].is_call() {
                let return_index = history.index_of(history[curr].id);
                let (is_valid, new_state) = self
                    .spec
                    .apply(history[return_index].action.clone(), state.clone());
                let mut tmp_linearized = linearized.clone();
                tmp_linearized[history[curr].id] = true;

                if !is_valid || !cache.insert((tmp_linearized, new_state.clone())) {
                    curr += 1;
                    continue;
                }
                linearized[history[curr].id] = true;
                let call = history.lift(curr);
                calls.push((call, state));
                state = new_state;
                curr = 0;
            } else {
                match calls.pop() {
                    None => {
                        return false;
                    }
                    Some(((call, response), old_state)) => {
                        state = old_state;
                        linearized[call.id] = false;
                        let (call_index, _) = history.unlift(call, response);
                        curr = call_index + 1;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use Action::*;

    #[derive(Copy, Clone, Debug)]
    enum RegisterOp {
        Write(usize),
        Read(usize),
    }

    use RegisterOp::*;

    struct IntegerRegisterSpec {}

    impl Specification for IntegerRegisterSpec {
        type State = usize;
        type Operation = RegisterOp;

        fn init(&self) -> Self::State {
            0
        }

        fn apply(
            &self,
            action: Action<Self::Operation>,
            state: Self::State,
        ) -> (bool, Self::State) {
            match action {
                Call(_) => (true, state),
                Response(op) => match op {
                    Write(value) => (true, value),
                    Read(value) => (value == state, state),
                },
            }
        }
    }

    mod is_linearizable {
        use super::*;

        #[test]
        fn test_accepts_sequential_read_and_write() {
            let checker = WLGChecker {
                spec: IntegerRegisterSpec {},
            };
            let history = History::from_actions(vec![
                (0, Call(Write(1))),
                (0, Response(Write(1))),
                (0, Call(Read(1))),
                (0, Response(Read(1))),
            ]);
            assert!(checker.is_linearizable(history));
        }

        #[test]
        fn test_rejects_invalid_reads() {
            let checker = WLGChecker {
                spec: IntegerRegisterSpec {},
            };
            let history = History::from_actions(vec![
                (0, Call(Write(1))),
                (0, Response(Write(1))),
                (0, Call(Read(2))),
                (0, Response(Read(2))),
            ]);
            assert!(!checker.is_linearizable(history));
        }

        // #[test]
        // fn test_accepts_invalid_reads() {
        //     let checker = WLGChecker {
        //         spec: IntegerRegisterSpec {},
        //     };
        //     let mut history = History::from_ops(vec![
        //         RegisterOp::Write(1),
        //         RegisterOp::Write(1),
        //         RegisterOp::Read(3),
        //         RegisterOp::Read(3),
        //     ]);
        //     history[0].response = Some(1);
        //     history[2].response = Some(3);
        //     assert!(!checker.is_linearizable(history));

        #[test]
        fn test_accepts_writes_in_reverse_writes() {
            // Accepts the following history, in which processes
            // P1, P2, and P3 must linearize their writes in the
            // reverse order in which they are called.
            // P0 |--------------------| Write(1)
            // P1 |--------------------| Write(2)
            // P2 |--------------------| Write(3)
            // P3   |--|                 Read(3)
            // P3          |--|          Read(2)
            // P3                 |--|   Read(1)
            let checker = WLGChecker {
                spec: IntegerRegisterSpec {},
            };
            let history = History::from_actions(vec![
                (0, Call(Write(1))),
                (1, Call(Write(2))),
                (2, Call(Write(3))),
                (3, Call(Read(3))),
                (3, Response(Read(3))),
                (3, Call(Read(2))),
                (3, Response(Read(2))),
                (3, Call(Read(2))),
                (3, Response(Read(2))),
                (0, Response(Write(1))),
                (1, Response(Write(2))),
                (3, Response(Write(3))),
            ]);
            assert!(checker.is_linearizable(history));
        }

        #[test]
        fn test_rejects_sequentially_consistent_reads() {
            // Rejects the following history, in which P1 and P2 read
            // different values while overlapping with P0s write. Notice
            // that this history is _sequentially consistent_, as P2s
            // read could be re-ordered to complete prior to any other
            // operation.
            // P0 |-------------------| Write(1)
            // P1      |--|             Read(1)
            // P2              |--|     Read(0)
            let checker = WLGChecker {
                spec: IntegerRegisterSpec {},
            };
            let history = History::from_actions(vec![
                (0, Call(Write(1))),
                (1, Call(Read(1))),
                (1, Response(Read(1))),
                (2, Call(Read(0))),
                (2, Response(Read(0))),
                (0, Response(Write(1))),
            ]);
            assert!(!checker.is_linearizable(history));
        }
    }
}
