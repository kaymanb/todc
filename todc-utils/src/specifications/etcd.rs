//! A sequential specification of an [etcd](https://etcd.io/) key-value store.
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use crate::linearizability::history::{Action, History};
use crate::specifications::Specification;

type ProcessID = usize;

/// Returns the contents of the file, line by line.
///
/// Recipe from: https://doc.rust-lang.org/rust-by-example/std_misc/file/read_lines.html
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

/// Returns a history of operations performed on a etcd server being
/// tested by [Jepsen](https://github.com/jepsen-io/jepsen).
///
/// The history is created by parsing logs from Jepsen. See
/// [here](https://github.com/kaymanb/todc/blob/main/todc-utils/tests/linearizability/etcd/etcd_000.log)
/// for an example of such a log file.
pub fn history_from_log(filename: String) -> History<EtcdOperation> {
    let mut unknowns: Vec<(ProcessID, Action<EtcdOperation>)> = Vec::new();
    let mut actions: Vec<(ProcessID, Action<EtcdOperation>)> = Vec::new();
    for line in read_lines(filename).unwrap() {
        let line = line.unwrap();
        let words: Vec<&str> = line.split_whitespace().collect();
        if words.len() < 7 {
            continue;
        };
        if words[1] != "jepsen.util" {
            continue;
        };
        if words[3] == ":nemesis" {
            continue;
        };

        let process: usize = words[3].parse().unwrap();
        // Logs are marked with :info when the success of the operation is unknown. It
        // suffices to consider a history where all such operation succeed, but where the
        // Okay response for each operation occurs at the very end of the history.
        // See: https://aphyr.com/posts/316-jepsen-etcd-and-consul#writing-a-client
        if words[4] == ":info" {
            let (_, call) = actions
                .iter()
                .rev()
                .find(|(pid, _)| *pid == process)
                .unwrap()
                .clone();
            let response = match call {
                Action::Call(operation) => match operation {
                    // Reads are a special case, in that they do not affect the state of the
                    // object. Instead of the operations success being unknown, they can simply
                    // be treated as having failed, and should be marked as such in the logs.
                    Read(_, _) => panic!("Success of read operation cannot be unknown"),
                    Write(_, value) => Write(Unknown, value),
                    CompareAndSwap(_, cas) => CompareAndSwap(Unknown, cas),
                },
                Action::Response(_) => {
                    panic!("Expected previous operation by process {process} to be a call")
                }
            };
            unknowns.push((process, Action::Response(response)));
            continue;
        }

        let status = EtcdStatus::from_log(words[4]);
        let operation = EtcdOperation::from_log(&words[4..]);
        let action = match status {
            EtcdStatus::Invoke => Action::Call(operation),
            _ => Action::Response(operation),
        };

        actions.push((process, action))
    }

    // Append responses for operations whose status was unknown to the end of the
    // history.
    for item in unknowns.into_iter() {
        actions.push(item);
    }
    History::from_actions(actions)
}

/// The status of an etcd operation.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum EtcdStatus {
    Invoke,
    Okay,
    Fail,
    Unknown,
}

impl EtcdStatus {
    fn from_log(string: &str) -> Self {
        if string == ":invoke" {
            Self::Invoke
        } else if string == ":ok" {
            Self::Okay
        } else if string == ":fail" {
            Self::Fail
        } else if string == ":info" {
            Self::Unknown
        } else {
            panic!("Unexpected status: '{string}'")
        }
    }
}

use EtcdStatus::*;

/// An etcd operation.
#[derive(Debug, Copy, Clone)]
pub enum EtcdOperation {
    Read(EtcdStatus, Option<u32>),
    Write(EtcdStatus, u32),
    CompareAndSwap(EtcdStatus, (u32, u32)),
}

impl EtcdOperation {
    fn from_log(words: &[&str]) -> Self {
        let status = EtcdStatus::from_log(words[0]);
        let operation = words[1];
        if operation == ":read" {
            let value = if words[2] == "nil" || words[2] == ":timed-out" {
                None
            } else {
                Some(words[2].parse::<u32>().unwrap())
            };
            Self::Read(status, value)
        } else if operation == ":write" {
            let value = words[2].parse::<u32>().unwrap();
            Self::Write(status, value)
        } else if operation == ":cas" {
            let value = (
                words[2][1..].parse().unwrap(),
                words[3][..1].parse().unwrap(),
            );
            Self::CompareAndSwap(status, value)
        } else {
            panic!("Unexpected operation: '{operation}'")
        }
    }
}

use EtcdOperation::*;

/// A sequential specification of an [etcd](https://etcd.io/) key-value store.
///
/// The specification allows for reads, writes, and compare-and-swap (CAS) operations to be
/// performed on a single shared register. In practice, etcd stores exposes many such registers,
/// each indexed by unique key.
pub struct EtcdSpecification;

impl Specification for EtcdSpecification {
    type State = Option<u32>;
    type Operation = EtcdOperation;

    fn init(&self) -> Self::State {
        None
    }

    fn apply(&self, operation: &Self::Operation, state: &Self::State) -> (bool, Self::State) {
        match operation {
            Read(status, value) => match status {
                Okay => (value == state, *state),
                Fail => (value != state, *state),
                _ => panic!("Cannot apply read that has not succeeded or failed"),
            },
            Write(status, value) => match status {
                // TODO: Explain this...
                Invoke => panic!("Cannot apply write that has only been invoked"),
                Okay => (true, Some(*value)),
                Fail => (true, *state),
                Unknown => (true, Some(*value)),
            },
            CompareAndSwap(status, (compare, swap)) => {
                let success = match state {
                    Some(value) => compare == value,
                    None => false,
                };
                match status {
                    Invoke => panic!("Cannot apply CAS that has only been invoked"),
                    Okay => (success, if success { Some(*swap) } else { *state }),
                    Fail => (!success, *state),
                    Unknown => (true, if success { Some(*swap) } else { *state }),
                }
            }
        }
    }
}
