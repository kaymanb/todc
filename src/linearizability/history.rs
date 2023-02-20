//! A history.
use std::collections::VecDeque;
use std::iter::repeat_with;
use std::ops::{Index, IndexMut};

type EntryID = usize;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Action<C, R> {
    Call(C),
    Response(R),
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct CallEntry<C> {
    pub id: EntryID,
    pub action: C,
    pub response: EntryID,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ResponseEntry<R> {
    pub id: EntryID,
    pub action: R,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Entry<C, R> {
    Call(CallEntry<C>),
    Response(ResponseEntry<R>),
}

impl<C, R> Entry<C, R> {
    pub fn id(&self) -> EntryID {
        match self {
            Entry::Call(entry) => entry.id,
            Entry::Response(entry) => entry.id,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct History<C, R> {
    pub entries: Vec<Entry<C, R>>,
    // When an entry is removed from this history, its index is recorded here.
    removed_from: Vec<Option<EntryID>>,
}

impl<C, R> History<C, R> {
    /// # Panics
    ///
    /// Panics if `actions` is empty.
    /// Panics if the resulting history would be incomplete.
    pub fn from_actions(actions: Vec<(usize, Action<C, R>)>) -> Self {
        let (processes, actions): (Vec<usize>, Vec<Action<C, R>>) = actions.into_iter().unzip();

        let num_processes = processes.iter().max().unwrap();
        let mut calls: Vec<VecDeque<usize>> = repeat_with(VecDeque::new)
            .take(*num_processes + 1)
            .collect();
        let mut responses = calls.clone();
        for (i, process) in processes.iter().enumerate() {
            match &actions[i] {
                Action::Call(_) => calls[*process].push_back(i),
                Action::Response(_) => responses[*process].push_back(i),
            }
        }

        Self {
            entries: actions
                .into_iter()
                .enumerate()
                .map(|(i, action)| match action {
                    Action::Call(c) => Entry::Call(CallEntry {
                        id: i,
                        action: c,
                        response: responses[processes[i]].pop_front().unwrap(),
                    }),
                    Action::Response(r) => Entry::Response(ResponseEntry { id: i, action: r }),
                })
                .collect(),
            removed_from: repeat_with(|| None).take(processes.len()).collect(),
        }
    }

    pub fn index_of_id(&self, id: EntryID) -> usize {
        self.iter().position(|e| e.id() == id).unwrap()
    }

    /// # Panics
    ///
    /// Panics if input entry was not previously removed from the history.
    fn insert(&mut self, entry: Entry<C, R>) -> usize {
        match self.removed_from[entry.id()].take() {
            Some(index) => {
                self.entries.insert(index, entry);
                index
            }
            None => panic!(
                "Index that entry {} was removed from is unknown",
                entry.id()
            ),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entry<C, R>> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn lift(&mut self, i: usize) -> (Entry<C, R>, Entry<C, R>) {
        match self.remove(i) {
            Entry::Response(_) => panic!("Cannot lift a response entry out of the history"),
            Entry::Call(call) => {
                let response = self.remove(self.index_of_id(call.response));
                (Entry::Call(call), response)
            }
        }
    }

    fn remove(&mut self, i: usize) -> Entry<C, R> {
        let entry = self.entries.remove(i);
        self.removed_from[entry.id()] = Some(i);
        entry
    }

    pub fn unlift(&mut self, call: Entry<C, R>, response: Entry<C, R>) -> (usize, usize) {
        let response_index = self.insert(response);
        let call_index = self.insert(call);
        (call_index, response_index)
    }
}

impl<C, R> Index<usize> for History<C, R> {
    type Output = Entry<C, R>;

    fn index(&self, i: usize) -> &Self::Output {
        self.entries.index(i)
    }
}

impl<C, R> IndexMut<usize> for History<C, R> {
    fn index_mut(&mut self, i: usize) -> &mut Self::Output {
        self.entries.index_mut(i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::zip;
    use Action::{Call, Response};

    mod from_actions {
        use super::*;

        #[test]
        fn creates_sequential_ids() {
            let history = History::from_actions(vec![
                (0, Call("a")),
                (0, Response("a")),
                (0, Call("b")),
                (0, Response("b")),
            ]);
            for (i, entry) in history.iter().enumerate() {
                assert_eq!(entry.id(), i);
            }
        }

        #[test]
        fn links_actions_of_multiple_processes() {
            let history = History::from_actions(vec![
                (0, Call("a")),
                (1, Call("b")),
                (2, Call("c")),
                (0, Response("a")),
                (1, Response("b")),
                (2, Response("c")),
                (0, Call("e")),
                (1, Call("f")),
                (2, Call("g")),
                (0, Response("e")),
                (1, Response("f")),
                (2, Response("g")),
            ]);
            for entry in history.iter() {
                println!("{:?}", entry);
                if let Entry::Call(call) = entry {
                    match &history[history.index_of_id(call.response)] {
                        Entry::Response(response) => assert_eq!(call.action, response.action),
                        Entry::Call(_) => panic!("Call entry was linked to another call entry"),
                    }
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
            for entry in history.iter() {
                if let Entry::Call(call) = entry {
                    match &history[history.index_of_id(call.response)] {
                        Entry::Response(response) => assert_eq!(call.action, response.action),
                        Entry::Call(_) => panic!("Call entry was linked to another call entry"),
                    }
                }
            }
        }
    }

    mod insert {
        use super::*;

        #[test]
        fn is_inverse_of_remove() {
            let history = History::from_actions(vec![
                (0, Call("a")),
                (1, Call("b")),
                (2, Call("c")),
                (0, Response("a")),
                (1, Response("b")),
                (2, Response("c")),
            ]);
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
                (2, Call("c")),
                (0, Response("a")),
                (1, Response("b")),
                (2, Response("c")),
            ]);
            history.lift(0);
            for (entry, letter) in zip(history.iter(), ["b", "c", "b", "c"]) {
                match entry {
                    Entry::Call(call) => assert_eq!(call.action, letter),
                    Entry::Response(resp) => assert_eq!(resp.action, letter),
                }
            }
        }
    }

    mod remove {
        use super::*;

        #[test]
        fn removes_only_requested_entry() {
            let mut history = History::from_actions(vec![
                (0, Call("a")),
                (1, Call("b")),
                (0, Response("a")),
                (1, Response("b")),
            ]);
            match history.remove(1) {
                Entry::Call(call) => assert_eq!(call.action, "b"),
                Entry::Response(_) => panic!("Removed incorrect entry"),
            }
            for (entry, letter) in zip(history.iter(), ["a", "a", "b"]) {
                match entry {
                    Entry::Call(entry) => assert_eq!(entry.action, letter),
                    Entry::Response(entry) => assert_eq!(entry.action, letter),
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
                (1, Response("b")),
            ]);
            let copy = history.clone();
            let (call, response) = history.lift(0);
            history.unlift(call, response);
            assert_eq!(history, copy)
        }
    }
}
