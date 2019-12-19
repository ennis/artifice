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

use crate::lens::{Lens, PartialPath, ComponentIndex};
use crate::lens::path::PartialPathSlice;

//mod trace;

//pub use crate::db::trace::Trace;
use std::cell::RefCell;
use std::ops::DerefMut;
use std::rc::{Rc, Weak};

pub trait Data: Clone + 'static {}
// blanket impl
impl<T: Clone + 'static> Data for T {}

/// Summary of changes to a collection.
///
/// Represents a summary of the modifications applied to a collection between two predefined points
/// in time.
#[derive(Clone,Debug,Eq,PartialEq)]
pub struct CollectionChanges {
    /// If this is an ordered collection, then `true` means that the layout of existing elements
    /// in the collection has changed (for instance, the collection was sorted).
    relayout: bool,

    /// For collections where elements have proper identity, contains the list of elements that were
    /// added to the collection.
    inserted_items: Vec<u64>,

    /// For collections where elements have proper identity, contains the list of elements that were
    /// removed from the collection.
    deleted_items: Vec<u64>,

    /// For collections where elements have proper identity, contains the list of elements that were
    /// modified.
    updated_items: Vec<u64>,
}


impl CollectionChanges {
    pub fn new() -> CollectionChanges {
        CollectionChanges {
            relayout: false,
            inserted_items: Vec::new(),
            deleted_items: Vec::new(),
            updated_items: Vec::new()
        }
    }

    pub fn merge_with(&mut self, other: CollectionChanges) {
        self.relayout |= other.relayout;
        self.inserted_items.extend(other.inserted_items);
        self.deleted_items.extend(other.deleted_items);
        self.updated_items.extend(other.updated_items);
    }
}

/// Describes a change applied to some data.
#[derive(Clone,Debug,Eq,PartialEq)]
pub enum Change {
    /// No changes
    None,
    /// Elements were inserted or removed into a collection.
    Collection(CollectionChanges),
    /// The entire element was replaced.
    Replace,
}

impl Change {
    pub fn replacement() ->  Change {
        Change::Replace
    }

    pub fn empty() -> Change {
        Change::None
    }

    fn get_collection_changes(&mut self) -> Option<&mut CollectionChanges> {
        if let Change::None = self {
            *self = Change::Collection(CollectionChanges::new());
        }

        if let Change::Collection(c) = self {
            Some(c)
        } else {
            None
        }
    }

    pub fn relayout(&mut self) -> &mut Self {
        if let Some(c) = self.get_collection_changes() {
            c.relayout = true;
        }
        self
    }

    pub fn insertion(&mut self, key: u64) -> &mut Self {
        if let Some(c) = self.get_collection_changes() {
            // TODO check duplicates
            c.inserted_items.push(key)
        }
        self
    }

    pub fn deletion(&mut self, key: u64) -> &mut Self {
        if let Some(c) = self.get_collection_changes() {
            // TODO check duplicates
            c.deleted_items.push(key)
        }
        self
    }

    pub fn update(&mut self, key: u64) -> &mut Self {
        if let Some(c) = self.get_collection_changes() {
            // TODO check duplicates
            c.updated_items.push(key)
        }
        self
    }

    pub fn merge_with(&mut self, other: Change) -> &mut Self {
        match other {
            Change::None => {},
            Change::Collection(c) => {
                if let Some(c2) = self.get_collection_changes() {
                    c2.merge_with(c)
                }
            }
            Change::Replace => {
                *self = Change::Replace
            }
        }
        self
    }
}


/// Represents an edit on a value of type T (i.e. a modification of part of the object).
pub trait Update<T: Data> {
    fn apply(&mut self, model: &mut T) -> Change;
    fn path(&self) -> PartialPath<T>;
}


/// Represents a partial change to an aggregate model.
pub struct Replace<K: Lens> {
    lens: K,
    value: K::Leaf,
}

impl<K: Lens> Replace<K> {
    pub fn new(lens: K, value: K::Leaf) -> Replace<K> {
        Replace { lens, value }
    }
}

impl<U, K> Update<U> for Replace<K>
where
    U: Data,
    K: Lens<Root = U>,
{
    fn apply(&mut self, model: &mut U) -> Change {
        *self.lens.get_mut(model) = self.value.clone();
        Change::replacement()
    }

    fn path(&self) -> PartialPath<K::Root> {
        self.lens.path().into()
    }
}

//--------------------------------------------------------------------------------------------------

/// Append operation.
pub struct Append<A: Data, K: Lens> {
    lens: K,
    element: A,
}

impl<A: Data, K: Lens> Append<A, K> {
    pub fn new(into: K, element: A) -> Append<A, K> {
        Append {
            lens: into,
            element,
        }
    }
}

// Append to Vec<A>
impl<A: Data, K: Lens<Leaf = Vec<A>>> Update<K::Root> for Append<A, K> {
    fn apply(&mut self, model: &mut K::Root) -> Change {
        self.lens.get_mut(model).push(self.element.clone());
        Change::replacement() // TODO more precise description ?
    }

    fn path(&self) -> PartialPath<K::Root> {
        self.lens.path().into()
    }
}

//--------------------------------------------------------------------------------------------------
/// Insert operation
pub struct Insert<A: Data, K: Lens, I: Clone> {
    lens: K,
    index: I,
    element: A,
}

impl<A: Data, K: Lens, I: Clone> Insert<A, K, I> {
    pub fn new(into: K, index: I, element: A) -> Insert<A, K, I> {
        Insert {
            lens: into,
            index,
            element,
        }
    }
}

// Insert into Vec<A>
impl<A: Data, K: Lens<Leaf = Vec<A>>> Update<K::Root> for Insert<A, K, usize> {
    fn apply(&mut self, model: &mut K::Root) -> Change {
        self.lens
            .get_mut(model)
            .insert(self.index, self.element.clone());
        Change::replacement()
    }

    fn path(&self) -> PartialPath<K::Root> {
        self.lens.path().into()
    }
}

//--------------------------------------------------------------------------------------------------

/// A view over a database.
pub trait View<Root: Data> {
    /// Called by the database when something has changed.
    fn on_change(&self, path: PartialPathSlice<Root>, change: &Change);
}

/// 'Database' wrapper for a data model that keeps track of changes in the model.
pub struct Database<M: Data> {
    data: RefCell<M>,
    log: RefCell<Vec<Box<dyn Update<M>>>>,
    views: RefCell<Vec<Weak<dyn View<M>>>>,
}

impl<M: Data> Database<M> {
    /// Creates a new database wrapping an existing data model instance.
    pub fn new(data: M) -> Database<M> {
        Database {
            data: RefCell::new(data),
            log: RefCell::new(Vec::new()),
            views: RefCell::new(Vec::new())
        }
    }

    pub fn update(&self, mut u: impl Update<M> + 'static) {
        let mut data = self.data.borrow_mut();
        // apply the update
        let change = u.apply(&mut *data);

        eprintln!("database update {:?}", u.path());

        // update all views
        let path = u.path();
        for view in self.views.borrow().iter() {
            if let Some(view) = view.upgrade() {
                view.on_change(path.as_slice(), &change);
            }
        }

        // and record it in the log
        self.log.borrow_mut().push(Box::new(u));
    }

    pub fn add_view(&mut self, view: Rc<dyn View<M>>) {
        self.views.borrow_mut().push(Rc::downgrade(&view))
    }

    /*/// Returns the current revision number.
    fn last_revision(&mut self) -> u64 {
        self.changes.last().map_or(0, |c| c.revision)
    }

    fn revisions_since(&self, revision: u64) -> &[Revision] {
        // find all changes with revision number > revision
        let revs = match self.revs.binary_search_by(|r| r.number.cmp(&revision)) {
            Ok(p) => &self.revs[p+1..],
            Err(p) => &self.revs[p..]
        };
        revs
    }

    /// Returns the list of changes since the specified revision number.
    pub fn trace_since_revision(&self, revision: u64) -> Trace<M> {
        Trace::new(self, revision)
    }*/
}


impl<M: Data> Database<M> {
    pub fn append<A, K>(&mut self, lens: K, element: A ) where
        A: Data,
        K: Lens,
        Append<A, K>: Update<M>
    {
        self.update(Append::new(lens,element))
    }

    pub fn replace<K: Lens<Root=M>>(&mut self, lens: K, element: K::Leaf) where
    {
        self.update(Replace::new(lens,element))
    }
}
