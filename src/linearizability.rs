//! Checking [linearizability](https://en.wikipedia.org/wiki/Linearizability).  
//!
//! See _Testing for Linearizability_ by Gavin Lowe [\[L16\]](https://doi.org/10.1002/cpe.3928) and
//! _Faster Linearizability Checking via P-Compositionality_ by Horn and Kroening
//! [\[HK15\]](https://arxiv.org/abs/1504.00204). For a Go implementation, see [Porcupine](https://github.com/anishathalye/porcupine).

use crate::linearizability::history::{Entry, History};
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;

pub mod history;

pub trait Specification {
    type State: Clone + Eq + Hash + Debug;
    type CallOp: Clone + Debug;
    type ResponseOp: Clone + Debug;

    /// Returns an initial state for the specification.
    fn init(&self) -> Self::State;

    /// Returns the state that results from a valid call and response pair. .
    fn apply(
        &self,
        call: Self::CallOp,
        response: Self::ResponseOp,
        state: Self::State,
    ) -> (bool, Self::State);
}

pub struct WLGChecker<S: Specification> {
    pub spec: S,
}

type SpecEntry<S> = Entry<<S as Specification>::CallOp, <S as Specification>::ResponseOp>;

impl<S: Specification> WLGChecker<S> {
    pub fn is_linearizable(&self, mut history: History<S::CallOp, S::ResponseOp>) -> bool {
        let mut state = self.spec.init();
        let mut linearized = vec![false; history.len()];
        // TODO: Figure out how to type alias Entry<S::CallOp, S::ResponseOp>
        let mut calls: Vec<((SpecEntry<S>, SpecEntry<S>), S::State)> = Vec::new();
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
                        // TODO: Better memory management so these clones aren't necessary.
                        let (is_valid, new_state) = self.spec.apply(
                            call.action.clone(),
                            response.action.clone(),
                            state.clone(),
                        );
                        let mut tmp_linearized = linearized.clone();
                        tmp_linearized[call.id] = true;
                        if !is_valid || !cache.insert((tmp_linearized, new_state.clone())) {
                            curr += 1;
                            continue;
                        }
                        linearized[call.id] = true;
                        let call = history.lift(curr);
                        calls.push((call, state));
                        state = new_state;
                        curr = 0;
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
    enum RegisterCall {
        WriteCall(usize),
        ReadCall,
    }

    #[derive(Copy, Clone, Debug)]
    enum RegisterResponse {
        ReadResp(usize),
        WriteResp(usize),
    }

    use RegisterCall::*;
    use RegisterResponse::*;

    struct IntegerRegisterSpec {}

    impl Specification for IntegerRegisterSpec {
        type State = usize;
        type CallOp = RegisterCall;
        type ResponseOp = RegisterResponse;

        fn init(&self) -> Self::State {
            0
        }

        fn apply(
            &self,
            call: Self::CallOp,
            response: Self::ResponseOp,
            state: Self::State,
        ) -> (bool, Self::State) {
            match call {
                WriteCall(c_value) => match response {
                    WriteResp(r_value) => {
                        let valid = c_value == r_value;
                        (valid, if valid { c_value } else { state })
                    }
                    _ => (false, state),
                },
                ReadCall => match response {
                    ReadResp(value) => (value == state, state),
                    _ => (false, state),
                },
            }
        }
    }

    mod is_linearizable {
        use super::*;

        #[test]
        fn accepts_sequential_read_and_write() {
            let checker = WLGChecker {
                spec: IntegerRegisterSpec {},
            };
            let history = History::from_actions(vec![
                (0, Call(WriteCall(1))),
                (0, Response(WriteResp(1))),
                (0, Call(ReadCall)),
                (0, Response(ReadResp(1))),
            ]);
            assert!(checker.is_linearizable(history));
        }

        #[test]
        fn rejects_invalid_reads() {
            let checker = WLGChecker {
                spec: IntegerRegisterSpec {},
            };
            let history = History::from_actions(vec![
                (0, Call(WriteCall(1))),
                (0, Response(WriteResp(1))),
                (0, Call(ReadCall)),
                (0, Response(ReadResp(2))),
            ]);
            assert!(!checker.is_linearizable(history));
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
            let checker = WLGChecker {
                spec: IntegerRegisterSpec {},
            };
            let history = History::from_actions(vec![
                (0, Call(WriteCall(1))),
                (1, Call(WriteCall(2))),
                (2, Call(WriteCall(3))),
                (3, Call(ReadCall)),
                (3, Response(ReadResp(3))),
                (3, Call(ReadCall)),
                (3, Response(ReadResp(2))),
                (3, Call(ReadCall)),
                (3, Response(ReadResp(1))),
                (0, Response(WriteResp(1))),
                (1, Response(WriteResp(2))),
                (2, Response(WriteResp(3))),
            ]);
            assert!(checker.is_linearizable(history));
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
            let checker = WLGChecker {
                spec: IntegerRegisterSpec {},
            };
            let history = History::from_actions(vec![
                (0, Call(WriteCall(1))),
                (1, Call(ReadCall)),
                (1, Response(ReadResp(1))),
                (2, Call(ReadCall)),
                (2, Response(ReadResp(0))),
                (0, Response(WriteResp(1))),
            ]);
            assert!(!checker.is_linearizable(history));
        }
    }
}
