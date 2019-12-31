//! # Database
//! The "source of truth", contains the state.
//!
//! A _revision_ is a snapshot of the database state at some point in time.
//! Revisions are identified by _revision numbers_.
//!
//! Note that there is only one revision available at a time (the latest one).
//! However, it is possible to rollback the database to the state of a previous revision with
//! undo operations. Note that this does not remove revisions: instead, it creates new revisions
//! that revert the changes (like git revert).
//!
//! # Trace
//! A _trace_ is a view of a part of the data in the database and a change that affected it.

use crate::data::Data;
use crate::lens::Lens;
use crate::update::{Append, Replace, Update};
use crate::{Change, Collection, CollectionChanges, IndexAddress};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

//--------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct Revision<'a, Root: Data> {
    data: &'a Root,
    addr: Option<Root::Address>,
    change: &'a Change,
}

// #26925 impl
impl<'a, Root: Data> Clone for Revision<'a, Root> {
    fn clone(&self) -> Self {
        Revision {
            addr: self.addr.clone(),
            data: self.data,
            change: self.change,
        }
    }
}

//
impl<'a, Root: Data> Revision<'a, Root> {
    pub fn focus<K: Lens<Root = Root>>(&self, lens: K) -> Option<Revision<'a, K::Leaf>> {
        if let Change::Replace = self.change {
            Some(Revision {
                change: &self.change,
                addr: None,
                data: lens.get(self.data),
            })
        } else {
            self.addr.clone().and_then(|addr| {
                lens.unprefix(addr).map(|suffix| Revision {
                    addr: suffix,
                    change: self.change,
                    data: lens.get(self.data),
                })
            })
        }
    }

    pub fn replaced(&self) -> bool {
        self.change == &Change::Replace
    }

    pub fn collection_changes(&self) -> Option<&CollectionChanges> {
        match self.change {
            Change::Collection(changes) => Some(changes),
            _ => None,
        }
    }

    pub fn address(&self) -> &Option<Root::Address> {
        &self.addr
    }

    /*pub fn change(&self) -> &Change {
        self.change
    }*/

    pub fn data(&self) -> &Root {
        self.data
    }
}

impl<'a, Root: Data> From<&'a Root> for Revision<'a, Root> {
    fn from(data: &'a Root) -> Self {
        static CHANGE: Change = Change::Replace;
        Revision {
            data,
            change: &CHANGE,
            addr: None,
        }
    }
}

impl<'a, Root> Revision<'a, Root>
where
    Root: Data + Collection,
    Root::Address: IndexAddress<Element = Root::Element, Index = Root::Index>,
{
    pub fn focus_index(&self, index: Root::Index) -> Option<Revision<'a, Root::Element>> {
        self.focus(<Root as Collection>::at_index(index))
    }
}

/// A view over a database.
pub trait Watcher<Root: Data> {
    /// Called by the database when something has changed.
    fn on_change(&self, revision: Revision<Root>);
}

/// 'Database' wrapper for a data model that keeps track of changes in the model.
pub struct Database<M: Data> {
    data: RefCell<M>,
    log: RefCell<Vec<Box<dyn Update<M>>>>,
    watchers: RefCell<Vec<Weak<dyn Watcher<M>>>>,
}

impl<M: Data> Database<M> {
    /// Creates a new database wrapping an existing data model instance.
    pub fn new(data: M) -> Database<M> {
        Database {
            data: RefCell::new(data),
            log: RefCell::new(Vec::new()),
            watchers: RefCell::new(Vec::new()),
        }
    }

    pub fn update(&self, mut u: impl Update<M> + 'static) {
        let mut data = self.data.borrow_mut();
        // apply the update
        let change = u.apply(&mut *data);

        eprintln!("database update {:?}", u.address());

        // update all views
        let addr = u.address();
        let rev = Revision {
            change: &change,
            data: &*data,
            addr: addr.clone(),
        };
        for w in self.watchers.borrow().iter() {
            if let Some(w) = w.upgrade() {
                w.on_change(rev.clone());
            }
        }

        // and record it in the log
        self.log.borrow_mut().push(Box::new(u));
    }

    pub fn add_watcher(&mut self, w: Rc<dyn Watcher<M>>) {
        let data = self.data.borrow();
        w.on_change((&*data).into());
        self.watchers.borrow_mut().push(Rc::downgrade(&w))
    }
}

impl<M: Data> Database<M> {
    pub fn append<A, K>(&mut self, lens: K, element: A)
    where
        A: Data,
        K: Lens,
        Append<A, K>: Update<M>,
    {
        self.update(Append::new(lens, element))
    }

    pub fn replace<K: Lens<Root = M>>(&mut self, lens: K, element: K::Leaf) {
        self.update(Replace::new(lens, element))
    }
}
