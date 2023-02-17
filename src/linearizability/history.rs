//! A history.
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
    pub action: R
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Entry<C, R> {
    Call(CallEntry<C>),
    Response(ResponseEntry<R>)
}

impl<C, R> Entry<C, R> {
    
    fn from_action(id: EntryID, action: Action<C, R>) -> Self {
        match action {
            Action::Call(action) => Self::Call(CallEntry {
                id,
                action,
                // HACK: This value shouldn't stay as id
                // TODO: Fix this next
                response: id
            }),
            Action::Response(action) => Self::Response(ResponseEntry {
                id,
                action,
            })
        }
    }

    pub fn id(&self) -> EntryID {
        match self {
            Entry::Call(entry) => entry.id,
            Entry::Response(entry) => entry.id
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
    pub fn from_actions(actions: Vec<(usize, Action<C, R>)>) -> Self {
        let (processes, actions): (Vec<usize>, Vec<Action<C, R>>) = actions.into_iter().unzip();
        let mut history = Self {
            entries: actions
                .into_iter()
                .enumerate()
                .map(|(i, action)| Entry::from_action(i, action))
                .collect(),
            removed_from: repeat_with(|| None).take(processes.len()).collect(),
        };

        // Link associated call and response entries within the history
        let num_processes = processes.iter().max().unwrap();
        let mut calls: Vec<Option<usize>> = repeat_with(|| None).take(*num_processes + 1).collect();
        // TODO: Validate that the history is complete.
        for (idx, process) in processes.iter().enumerate() {
            match &history[idx] {
                Entry::Call(_) => {
                    calls[*process] = Some(idx);
                }
                Entry::Response(response) => {
                    let entry = &history[calls[*process].unwrap()];
                    match entry {
                        Entry::Call(call) => call.response = response.id,
                        Entry::Response(_) => panic!("")
                    }
                }
            }
        }
        return history;
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
            None => panic!("Index that entry {} was removed from is unknown", entry.id()),
        }
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
    use Action::{Call, Response};
    use std::iter::zip;

    mod from_actions {
        use super::*;

        #[test]
        fn creates_sequential_ids() {
            let history: History<&str, &str> =
                History::from_actions(vec![(0, Call("a")), (1, Call("b")), (2, Call("c"))]);
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
                (3, Call("d")),
                (3, Response("d")),
                (2, Response("c")),
                (1, Response("b")),
                (0, Response("a")),
            ]);
            for entry in history.iter() {
                if let Entry::Call(call) = entry {
                    match history[history.index_of_id(call.response)] {
                        Entry::Response(response) => assert_eq!(call.action, response.action),
                        Entry::Call(_) => panic!("Call entry was linked to another call entry")
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
                    match history[history.index_of_id(call.response)] {
                        Entry::Response(response) => assert_eq!(call.action, response.action),
                        Entry::Call(_) => panic!("Call entry was linked to another call entry")
                    }
                }
            }
        }
    }

    mod insert {
        use super::*;
        
        #[test]
        fn is_inverse_of_remove() {
            let history: History<&str, &str> =
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
                match entry {
                    Entry::Call(call) => assert_eq!(call.action, letter),
                    Entry::Response(_) => panic!("Unexpected response entry")
                }
            }
        }
    }

    mod remove {
        use super::*;

        #[test]
        fn removes_only_requested_entry() {
            let mut history: History<&str, &str> =
                History::from_actions(vec![(0, Call("a")), (1, Call("b")), (2, Call("c"))]);
            
            match history.remove(1) {
                Entry::Call(call) => assert_eq!(call.action, "b"),
                Entry::Response(_) => panic!("Removed incorrect entry")
            }
            for (entry, letter) in zip(history.iter(), ["a", "c", "d"]) {
                match entry {
                    Entry::Call(entry) => assert_eq!(entry.action, letter),
                    Entry::Response(entry) => panic!("Unexpected response entry")
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
