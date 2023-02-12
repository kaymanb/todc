//! A history.
use std::iter::repeat_with;
use std::ops::{Index, IndexMut};

type EntryID = usize;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Action<T> {
    Call(T),
    Response(T),
}

use Action::*;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Entry<T> {
    pub id: EntryID,
    pub action: Action<T>,
    pub response: Option<EntryID>,
}

impl<T> Entry<T> {
    pub fn is_call(&self) -> bool {
        self.response.is_some()
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct History<T> {
    pub entries: Vec<Entry<T>>,
    // When an entry is removed from this history, its index is recorded here.
    removed_from: Vec<Option<usize>>,
}

impl<T> History<T> {
    /// # Panics
    ///
    /// Panics if `actions` is empty.
    pub fn from_actions(actions: Vec<(usize, Action<T>)>) -> Self {
        let (processes, actions): (Vec<usize>, Vec<Action<T>>) = actions.into_iter().unzip();
        let mut history = Self {
            entries: actions
                .into_iter()
                .enumerate()
                .map(|(i, action)| Entry {
                    id: i,
                    action,
                    response: None,
                })
                .collect(),
            removed_from: repeat_with(|| None).take(processes.len()).collect(),
        };

        // Link associated call and response actions within the history
        let num_processes = processes.iter().max().unwrap();
        let mut calls: Vec<Option<usize>> = repeat_with(|| None).take(*num_processes + 1).collect();
        // TODO: Validate that the history is valid and complete.
        for (idx, process) in processes.iter().enumerate() {
            match &history[idx].action {
                Call(_) => {
                    calls[*process] = Some(idx);
                }
                Response(_) => history[calls[*process].unwrap()].response = Some(history[idx].id),
            }
        }
        return history;
    }

    pub fn index_of(&self, id: EntryID) -> usize {
        self.iter().position(|e| e.id == id).unwrap()
    }

    /// # Panics
    ///
    /// Panics if input entry was not previously removed from the history.
    fn insert(&mut self, entry: Entry<T>) -> usize {
        match self.removed_from[entry.id].take() {
            Some(index) => {
                self.entries.insert(index, entry);
                index
            }
            None => panic!("Index that entry {} was removed from is unknown", entry.id),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entry<T>> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn lift(&mut self, i: usize) -> (Entry<T>, Entry<T>) {
        let call_entry = self.remove(i);
        // Use types to prevent these unwraps.
        let return_index = self.index_of(call_entry.response.unwrap());
        let return_entry = self.remove(return_index);
        (call_entry, return_entry)
    }

    fn remove(&mut self, i: usize) -> Entry<T> {
        let entry = self.entries.remove(i);
        self.removed_from[entry.id] = Some(i);
        entry
    }

    pub fn unlift(&mut self, call: Entry<T>, response: Entry<T>) -> (usize, usize) {
        let response_index = self.insert(response);
        let call_index = self.insert(call);
        (call_index, response_index)
    }
}

impl<T> Index<usize> for History<T> {
    type Output = Entry<T>;

    fn index(&self, i: usize) -> &Self::Output {
        self.entries.index(i)
    }
}

impl<T> IndexMut<usize> for History<T> {
    fn index_mut(&mut self, i: usize) -> &mut Self::Output {
        self.entries.index_mut(i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::zip;

    mod from_actions {
        use super::*;

        #[test]
        fn creates_sequential_ids() {
            let history =
                History::from_actions(vec![(0, Call("a")), (1, Call("b")), (2, Call("c"))]);
            for (i, entry) in history.iter().enumerate() {
                assert_eq!(entry.id, i);
            }
        }

        #[test]
        fn links_actions_of_multiple_processes() {
            let history = History::from_actions(vec![
                (0, Call("a")),
                (1, Call("b")),
                (2, Call("c")),
                (3, Call("d")),
                (3, Response("d")),
                (2, Response("c")),
                (1, Response("b")),
                (0, Response("a")),
            ]);
            for entry in history.clone().iter() {
                match entry.action {
                    Call(call_letter) => {
                        let idx = history.index_of(entry.response.unwrap());
                        match history[idx].action {
                            Call(_) => panic!("Call action cannot be a response"),
                            Response(response_letter) => assert_eq!(call_letter, response_letter)
                        }
                    },
                    Response(_) => assert!(entry.response.is_none()),
                }
            }
        }

        #[test]
        fn links_actions_of_single_process() {
            let history = History::from_actions(vec![
                (0, Call("a")),
                (0, Response("a")),
                (0, Call("b")),
                (0, Response("b")),
                (0, Call("c")),
                (0, Response("c")),
            ]);
            for entry in history.clone().iter() {
                match entry.action {
                    Call(call_letter) => {
                        let idx = history.index_of(entry.response.unwrap());
                        match history[idx].action {
                            Call(_) => panic!("Call action cannot be a response"),
                            Response(response_letter) => assert_eq!(call_letter, response_letter)
                        }
                    },
                    Response(_) => assert!(entry.response.is_none()),
                }
            }
        }
    }

    mod insert {
        use super::*;

        #[test]
        fn is_inverse_of_remove() {
            let history =
                History::from_actions(vec![(0, Call("a")), (1, Call("b")), (2, Call("c"))]);
            let mut copy = history.clone();
            let entry = copy.remove(1);
            copy.insert(entry);
            for (copy, entry) in zip(copy.iter(), history.iter()) {
                assert_eq!(copy, entry);
            }
        }
    }

    mod lift {
        use super::*;

        #[test]
        fn removes_call_and_response_entries() {
            let mut history = History::from_actions(vec![
                (0, Call("a")),
                (1, Call("b")),
                (0, Response("a")),
                (2, Call("c")),
            ]);
            history.lift(0);
            for (entry, letter) in zip(history.iter(), ["b", "c"]) {
                match entry.action {
                    Call(value) => assert_eq!(value, letter),
                    Response(_) => panic!("Unexpected action"),
                }
            }
        }
    }

    mod remove {
        use super::*;

        #[test]
        fn removes_only_requested_entry() {
            let mut history =
                History::from_actions(vec![(0, Call("a")), (1, Call("b")), (2, Call("c"))]);
            assert_eq!(Call("b"), history.remove(1).action);
            for (entry, letter) in zip(history.iter(), ["a", "c", "d"]) {
                match entry.action {
                    Call(value) => assert_eq!(value, letter),
                    Response(_) => panic!("Unexpected action"),
                }
            }
        }
    }

    mod unlift {
        use super::*;

        #[test]
        fn is_inverse_of_lift() {
            let mut history = History::from_actions(vec![
                (0, Call("a")),
                (1, Call("b")),
                (0, Response("a")),
                (2, Call("c")),
            ]);
            let copy = history.clone();
            let (call, response) = history.lift(0);
            history.unlift(call, response);
            assert_eq!(history, copy)
        }
    }
}
