use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use todc::linearizability::{WLGChecker, Specification};
use todc::linearizability::history::{Entry, History};

/// See https://doc.rust-lang.org/rust-by-example/std_misc/file/read_lines.html
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn history_from_log(filename: &str, num_processes: usize) -> History<EtcdOperation>{
    let mut count = 0;
    let mut entries: Vec<Entry<EtcdOperation>> = Vec::new();
    let mut calls: Vec<Option<usize>> = vec![None; num_processes + 1];
    for line in read_lines(filename).unwrap() {
        let line = line.unwrap();
        let words: Vec<&str> = line.split_whitespace().collect();
        if words.len() < 7 { continue };
        if words[1] != "jepsen.util" { continue };
        if words[3] == ":nemesis" { continue };
        
        let process: usize = words[3].parse().unwrap();
        let status = EtcdStatus::from_log(words[4]);
        let entry = Entry {
            id: count,
            operation: EtcdOperation::from_log(&words[4..]),
            rtrn: None,
            index: None
        }; 
        
        if status == EtcdStatus::Invoke {
            match calls[process] {
                None => calls[process] = Some(count),
                Some(_) => panic!("Process invoked multiple operations without response")
            }
        } else {
            match calls[process] {
                Some(index) => entries[index].rtrn = Some(count),
                None => panic!("Process returned from non-existent call")
            }
            calls[process] = None;

        }
        entries.push(entry);
        count += 1;
            
    }
    History {
        entries
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum EtcdStatus {
    Invoke,
    Okay,
    Fail,
    Info
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
            Self::Info
        } else {
            panic!("Unexpected status for etcd operation")
        }
    }
}
    
#[derive(Debug, Copy, Clone)]
enum EtcdOperation {
    Read(EtcdStatus, Option<u32>),
    Write(EtcdStatus, Option<u32>),
    CompareAndSwap(EtcdStatus, Option<(u32, u32)>)
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
            let value = if words [2] == ":timed-out" {
                None
            } else {
                Some(words[2].parse::<u32>().unwrap())
            };
            Self::Write(status, value)
        } else if operation == ":cas" {
            let value: Option<(u32, u32)> = if words[2] == ":timed-out" {
                None
            } else {
                Some((words[2][1..].parse().unwrap(), words[3][..1].parse().unwrap()))
            };
            Self::CompareAndSwap(status, value)
        } else {
            panic!("Unexpected operation for etcd operation")
        }
    }
}

struct EtcdSpecification {}

impl Specification for EtcdSpecification {
    type State = u32;
    type Operation = EtcdOperation;

    fn init(&self) -> Self::State {
        0
    }

    fn apply(&self, operation: Self::Operation, state: Self::State) -> (bool, Self::State) {
        match operation {
            EtcdOperation::Read(status, value) => {
                match status {
                    EtcdStatus::Okay => {
                        match value {
                            Some(value) => (value == state, state),
                            None => (false, state)
                        }
                    },
                    _ => (true, state)
                }
            },
            EtcdOperation::Write(status, value) => {
                match status {
                    EtcdStatus::Okay => {
                        match value {
                            Some(value) => (true, value),
                            None => panic!("Valid writes must apply a value")
                        }
                    },
                    _ => (true, state)
                }
            },
            EtcdOperation::CompareAndSwap(status, value) => {
                match status {
                    EtcdStatus::Okay => {
                        match value {
                            Some((compare, swap)) => {
                                if compare == state { (true, swap) }
                                else { (true, state) }
                            }
                            None => panic!("Valid CAS must include values")
                        }
                    },
                    _ => (true, state)
                }
            }

        } 
    }
}

#[cfg(test)]
mod tests {
    use super::*; 

    #[test]
    fn verifies_log_000_is_not_linearizable() {
        let checker = WLGChecker {
            spec: EtcdSpecification {}
        };        
        let result = checker.is_linearizable(history_from_log("tests/linearizability/etcd/etcd_000.log", 40));
        assert_eq!(result, false);
    }

    #[test]
    fn verifies_log_007_is_linearizable() {
        let checker = WLGChecker {
            spec: EtcdSpecification {}
        };        
        let result = checker.is_linearizable(history_from_log("tests/linearizability/etcd/etcd_007.log", 40));
        assert_eq!(result, true);
    }
}
