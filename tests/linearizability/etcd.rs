use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use todc::linearizability::history::{Action, History};
use todc::linearizability::{Specification, WLGChecker};

/// See https://doc.rust-lang.org/rust-by-example/std_misc/file/read_lines.html
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

type ProcessID = usize;

fn history_from_log(filename: &str) -> History<EtcdOperation> {
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
            let (_, call) = actions.iter().rev().find(|(pid, _)| *pid == process).unwrap().clone();
            let response = match call {
                Action::Call(operation) => match operation {
                    // Reads are a special case, in that they do not affect the state of the
                    // object. Instead of the operations success being unknown, they can simply
                    // be treated as having failed, and should be marked as such in the logs.
                    Read(_, _) => panic!("Success of read operation cannot be unknown"),
                    Write(_, value) => Write(Unknown, value),
                    CompareAndSwap(_, cas) => CompareAndSwap(Unknown, cas)
                },
                Action::Response(_) => panic!("Expected previous operation by process {} to be a call", process)
            };
            unknowns.push((process, Action::Response(response)));
            continue;
        }

        let status = EtcdStatus::from_log(words[4]);
        let operation = EtcdOperation::from_log(&words[4..]);
        let action = match status {
            EtcdStatus::Invoke => Action::Call(operation),
            _ => Action::Response(operation)
        };

        actions.push((process, action))
    }
    
    // Append responses for operations whose status was unknown to the end of the 
    // history.
    for item in unknowns.into_iter() {
        actions.push(item);
    }
    let x = History::from_actions(actions);
    for item in x.iter() {
        println!("{:?}", item);
    }
    x
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum EtcdStatus {
    Invoke,
    Okay,
    Fail,
    Unknown
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
            panic!("Unexpected status: '{}'", string)
        }
    }
}

use EtcdStatus::*;

#[derive(Debug, Copy, Clone)]
enum EtcdOperation {
    Read(EtcdStatus, Option<u32>),
    Write(EtcdStatus, u32),
    CompareAndSwap(EtcdStatus, (u32, u32))
}


impl EtcdOperation {
    fn from_log(words: &[&str]) -> Self {
        let status = EtcdStatus::from_log(words[0]);
        let operation = words[1];
        if operation == ":read" {
            let value = if words[2] == "nil" || words[2] == ":timed-out" {
                None
            } else {
                println!("{:?}", words[2]);
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
            panic!("Unexpected operation: '{}'", operation)
        }
    }
}

use EtcdOperation::*;

struct EtcdSpecification {}

impl Specification for EtcdSpecification {
    type State = Option<u32>;
    type Operation = EtcdOperation;

    fn init(&self) -> Self::State {
        None
    }

    fn apply(
        &self,
        operation: Self::Operation,
        state: Self::State,
    ) -> (bool, Self::State) {
        match operation {
            Read(status, value) => match status {
                Okay => (value == state, state),
                Fail => (value != state, state),
                _ => panic!("Cannot apply read that has not succeeded or failed"),
            },
            Write(status, value) => match status {
                Invoke => panic!("Cannot apply write that has only been invoked"),
                Okay => (true, Some(value)),
                Fail => (true, state),
                Unknown => (true, Some(value))
            },
            CompareAndSwap(status, (compare, swap)) => {
                let success = match state {
                    Some(value) => compare == value,
                    None => false
                };
                match status {
                    Invoke => panic!("Cannot apply CAS that has only been invoked"),
                    Okay => (success, if success { Some(swap) } else { state }),
                    Fail => (!success, state),
                    Unknown => (true, if success { Some(swap) } else { state })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[macro_export]
    macro_rules! etcd_tests {
        ( $($name:ident: $values:expr,)* )=> {
            $(
                #[test]
                fn $name() {
                    let (log_number, expected_result) = $values;
                    let checker = WLGChecker {
                        spec: EtcdSpecification {},
                    };
                    let result = checker.is_linearizable(history_from_log(
                        format!("tests/linearizability/etcd/etcd_{}.log", log_number).as_str()
                    ));
                    assert_eq!(result, expected_result);
                }
            )*
        }
    }

    etcd_tests! {
        test_000: ("000", false),
        test_001: ("001", false),
        test_002: ("002", true),
        test_003: ("003", false),
        test_004: ("004", false),
        test_005: ("005", true),
        test_006: ("006", false),
        test_007: ("007", true),
        test_008: ("008", false),
        test_009: ("009", false),
        test_010: ("010", false),
        test_011: ("011", false),
        test_012: ("012", false),
        test_013: ("013", false),
        test_014: ("014", false),
        test_015: ("015", false),
        test_016: ("016", false),
        test_017: ("017", false),
        test_018: ("018", true),
        test_019: ("019", false),
        test_020: ("020", false),
        test_021: ("021", false),
        test_022: ("022", false),
        test_023: ("023", false),
        test_024: ("024", false),
        test_025: ("025", true),
        test_026: ("026", false),
        test_027: ("027", false),
        test_028: ("028", false),
        test_029: ("029", false),
        test_030: ("030", false),
        test_031: ("031", true),
        test_032: ("032", false),
        test_033: ("033", false),
        test_034: ("034", false),
        test_035: ("035", false),
        test_036: ("036", false),
        test_037: ("037", false),
        test_038: ("038", true),
        test_039: ("039", false),
        test_040: ("040", false),
        test_041: ("041", false),
        test_042: ("042", false),
        test_043: ("043", false),
        test_044: ("044", false),
        test_045: ("045", true),
        test_046: ("046", false),
        test_047: ("047", false),
        test_048: ("048", true),
        test_049: ("049", true),
        test_050: ("050", false),
        test_051: ("051", true),
        test_052: ("052", false),
        test_053: ("053", true),
        test_054: ("054", false),
        test_055: ("055", false),
        test_056: ("056", true),
        test_057: ("057", false),
        test_058: ("058", false),
        test_059: ("059", false),
        test_060: ("060", false),
        test_061: ("061", false),
        test_062: ("062", false),
        test_063: ("063", false),
        test_064: ("064", false),
        test_065: ("065", false),
        test_066: ("066", false),
        test_067: ("067", true),
        test_068: ("068", false),
        test_069: ("069", false),
        test_070: ("070", false),
        test_071: ("071", false),
        test_072: ("072", false),
        test_073: ("073", false),
        test_074: ("074", false),
        test_075: ("075", true),
        test_076: ("076", true),
        test_077: ("077", false),
        test_078: ("078", false),
        test_079: ("079", false),
        test_080: ("080", true),
        test_081: ("081", false),
        test_082: ("082", false),
        test_083: ("083", false),
        test_084: ("084", false),
        test_085: ("085", false),
        test_086: ("086", false),
        test_087: ("087", true),
        test_088: ("088", false),
        test_089: ("089", false),
        test_090: ("090", false),
        test_091: ("091", false),
        test_092: ("092", true),
        test_093: ("093", false),
        test_094: ("094", false),
        // Etcd fails to boot during test case 95
        // test_095: ("095", None),
        test_096: ("096", false),
        test_097: ("097", false),
        test_098: ("098", true),
        test_099: ("099", false),
        test_100: ("100", true),
        test_101: ("101", true),
        test_102: ("102", true),
    }
}
