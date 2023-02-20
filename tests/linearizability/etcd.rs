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

fn history_from_log(filename: &str) -> History<EtcdCall, EtcdResponse> {
    let mut actions: Vec<(ProcessID, Action<EtcdCall, EtcdResponse>)> = Vec::new();
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
        let status = EtcdStatus::from_log(words[4]);
        let action = match status {
            EtcdStatus::Invoke => Action::Call(EtcdCall::from_log(&words[4..])),
            _ => Action::Response(EtcdResponse::from_log(&words[4..]))
        };
        actions.push((process, action))
    }
    History::from_actions(actions)
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum EtcdStatus {
    Invoke,
    Okay,
    Fail,
    Timeout,
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
            // All :info statuses indicate that the operation timed out.
            Self::Timeout
        } else {
            panic!("Unexpected status for etcd operation")
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum EtcdCall {
    ReadCall(EtcdStatus),
    WriteCall(EtcdStatus, u32),
    CASCall(EtcdStatus, (u32, u32))
}

impl EtcdCall {
    fn from_log(words: &[&str]) -> Self {
        let status = EtcdStatus::from_log(words[0]);
        // TODO: There must be a way to enforce this with types...
        if status != EtcdStatus::Invoke {
            panic!("All call statuses must be :invoke")
        }
        let operation = words[1];
        if operation == ":read" {
            Self::ReadCall(status)
        } else if operation == ":write" {
            Self::WriteCall(status, words[2].parse().unwrap())
        } else if operation == ":cas" {
            Self::CASCall(status,
                (
                    words[2][1..].parse().unwrap(),
                    words[3][..1].parse().unwrap(),
                )
            )
        } else {
            panic!("Unexpected operation: '{}'", operation)
        }
    }
}

use EtcdCall::*;

#[derive(Debug, Copy, Clone)]
enum EtcdResponse {
    ReadResp(EtcdStatus, Option<u32>),
    WriteResp(EtcdStatus, Option<u32>),
    CASResp(EtcdStatus, Option<(u32, u32)>)
}

impl EtcdResponse {
    fn from_log(words: &[&str]) -> Self {
        let status = EtcdStatus::from_log(words[0]);
        if status == EtcdStatus::Invoke {
            panic!("Etcd response cannot have status :invoke")
        }
        let operation = words[1];
        if operation == ":read" {
            let value = if words[2] == "nil" || words[2] == ":timed-out" {
                None
            } else {
                Some(words[2].parse::<u32>().unwrap())
            };
            Self::ReadResp(status, value)
        } else if operation == ":write" {
            let value = if words[2] == ":timed-out" {
                None
            } else {
                Some(words[2].parse::<u32>().unwrap())
            };
            Self::WriteResp(status, value)
        } else if operation == ":cas" {
            let value: Option<(u32, u32)> = if words[2] == ":timed-out" {
                None
            } else {
                Some((
                    words[2][1..].parse().unwrap(),
                    words[3][..1].parse().unwrap(),
                ))
            };
            Self::CASResp(status, value)
        } else {
            panic!("Unexpected operation: '{}'", operation)
        }
    }
}

use EtcdResponse::*;

struct EtcdSpecification {}

impl Specification for EtcdSpecification {
    type State = Option<u32>;
    type CallOp = EtcdCall;
    type ResponseOp = EtcdResponse;

    fn init(&self) -> Self::State {
        None
    }

    fn apply(
        &self,
        call: Self::CallOp,
        response: Self::ResponseOp,
        state: Self::State,
    ) -> (bool, Self::State) {
        match call {
            ReadCall(_status) => {
                if let ReadResp(status, value) = response {
                    match status {
                        EtcdStatus::Okay => (value == state, state),
                        // Reads marked :fail have actually timed out, but the result is the same.
                        EtcdStatus::Fail => (true, state),
                        _ => (false, state)
                    }
                // TODO: Remove the need for this else block
                } else {
                    panic!("Unexpected response")
                }
            },
            WriteCall(_status, _value) => {
                if let WriteResp(status, value) = response {
                    match status {
                        EtcdStatus::Okay => match value {
                            Some(value) => (true, Some(value)),
                            None => panic!("Valid writes must apply a value"),
                        },
                        EtcdStatus::Timeout => (true, state),
                        _ => (false, state)
                    }
                } else {
                    panic!("Unexpected response")
                }
            },
            CASCall(_status, call_value) => {
                if let CASResp(status, value) = response {
                    match status {
                        EtcdStatus::Okay => match value {
                            Some((compare, swap)) => match state {
                                Some(inner) => {
                                    if compare == inner {
                                        (true, Some(swap))
                                    } else {
                                        (false, state) // Because we expected to succeed
                                    }
                                }
                                None => (false, state),
                            },
                            None => panic!("Valid CAS must include values"),
                        },
                        // TODO: I've been handling timeouts wrong: they can be applied at _any_
                        // point in the future, not just between responses. 
                        EtcdStatus::Timeout => {
                            let (compare, swap) = call_value;
                            match state {
                                Some(inner) => {
                                    if compare == inner {
                                        (true, Some(swap))
                                    } else {
                                        (true, state) // Because response set no expectation
                                    }
                                },
                                // TODO: Should this be true?
                                None => (true, state),
                            }
                        },
                        EtcdStatus::Fail => match value {
                            Some((compare, _swap)) => match state {
                                Some(inner) => {
                                    if compare == inner {
                                        (false, state)
                                    } else {
                                        (true, state) // Because we expected to fail
                                    }
                                },
                                _ => (true, state)
                            },
                            None => panic!("Valid CAS must include values"),
                        },
                        _ => panic!("Unexpected Invoke")
                    }
                } else {
                    panic!("Unexpected response")
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
        // This is a "mini" test containing a snippit of 002
        // test_109: ("109", true),
    }
}
