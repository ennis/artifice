use sequence_trie::SequenceTrie;
use crate::db::{Data, Database, Revision, Change};
use crate::lens::{ComponentIndex, Lens};

//--------------------------------------------------------------------------------------------------


// bikeshedding:
// - Trace: summary of changes to a database
// - TraceScope: log scoped to a particular element
// - Changes: summary of changes to a particular element in a database

/// Summary of changes to a database.
pub struct Trace<'a, D: Data> {
    data: &'a D,
    trie: SequenceTrie<ComponentIndex, Vec<OperationType>>
}

impl<'a, D: Data> Trace<'a, D> {
    pub(super) fn new(db: &'a Database<D>, revision: u64) -> Trace<'a, D> {
        let revs = db.revisions_since(revision);
        let mut trie = SequenceTrie::<ComponentIndex, Vec<OperationType>>::new();

        for r in revs.iter() {
            if let Some(change) = trie.get_mut(&c.path[..]) {
                let change : &mut Change = change;
                match (ops, r.op) {
                    (Changes::Replace, _) => {
                        // can't do worse than a full replace
                    },
                    (Changes::Collection(_), OperationType::Replace) => {
                        *ops = Changes::Replace;
                    },
                    (Changes::Collection(_), OperationType::Relayout) => {
                        *ops = Changes::Replace;
                    },
                    (Changes::Collection(changes), OperationType::Insert(key)) => {
                        changes.push(CollectionChange::Insert(key));
                    },
                    (Changes::Collection(changes), OperationType::Remove(key)) => {
                        changes.push(CollectionChange::Remove(key));
                    },
                    (Changes::Collection(changes), OperationType::Modify) => {
                        unimplemented!()
                    },
                    (Changes::Relayout, OperationType::Relayout) => {
                        // no change
                    }
                    (Changes::Relayout, _) => {
                        *ops = Changes::Replace;
                    }
                }
                ops.push(c.op);
            }
            else {
                trie.insert(&c.path[..], vec![c.op]);
            }
        }

        dbg!(&trie);

        Trace {
            trie,
            data: db.data()
        }
    }

    pub fn root(&self) -> TraceScope<D> {
        TraceScope {
            trie: &self.trie,
            data: self.data
        }
    }
}

// for each element in the db, want to know:
// - if it changed or not since a given revision
//  - if it did change, want to know how it changed:
//      - elements added or removed
//      - layout of the collection changed
//      - value replaced
//      - sub-component changed


pub struct TraceScope<'a, D: Data> {
    data: &'a D,
    trie: &'a SequenceTrie<ComponentIndex, Change>
}

impl<'a, D: Data> TraceScope<'a, D>
{
    /// Returns a trace that focuses on changes to a specific part of the object identified
    /// by the lens, or None if no changes happened to the specified part.
    pub fn focus<L: Lens<Root=D>>(&self, lens: L) -> Option<TraceScope<L::Leaf>> {
        self.trie.get_node(lens.path().indices()).map(|trie| {
            TraceScope {
                trie,
                data: lens.get(self.data)
            }
        })
    }


    /// Returns whether the current value was entirely replaced.
    pub fn was_replaced(&self) -> bool {
        unimplemented!()
    }

    /// Returns the index / key of the element that was changed.
    pub fn changed_elements(&self)

    pub fn changes(&self) -> Changes {
        self.trie.value().map(|v| &v[..]).unwrap_or(&[])

        // several options:
        // - collapsed list of insertions/deletions (by key)
        // - modification (go deeper)
        // - set

    }
}


// TODO: View<M>, which is a view on a part of a database
// db.since(revision) -> db::View<Model>
//
// -> should the view see the list of changes, or apply changes one by one?

// problem:
// changes are only meaningful between the revisions just before and just after the change, not in
// subsequent revisions.
// (or at least, for lists with linear indices)
//
// possible solutions:
// - persistent indices (see Qt)
//      - note: immutable containers get this right (previous version still indexable)
//          - also: free undo-redo with immutable containers
//
// - don't record insertion/deletions in sequence containers (Vec, Deque, etc.)
//      - replace the whole container
//      - after all, if inserting an element at position p, all elements in [p..] should be considered invalid (all indices shifted)
//      - most collections will have stable element identity (i.e. will actually be maps)
//          - operation is insert element (somewhere in the collection) with specific "index" (not the sequence index, but the key),
//               don't care about the position
//
// - ensure that all dependents are updated after every atomic change
//      - not feasible?
//      - actually, why not?
//          - all DB updates are done in a context that is _observed_ by all observers.
//      - `db.transaction(|model| { ... }, views...)`
//      - locally (in scope) "bind" the views to the DB (so that they are mutably borrowed only when needed)
//      - OR: the DB owns the views
//

// Changes, operations, changeoperations, ChangeKind, Revision, ChangeRecord, etc.

// The command to change the value: Update
// How the value was changed:

// don't really like the way changes are tracked:
// - record a list of changes
// - when a diff is requested, read the changes and construct a trace
//
// The trace is a complex structure (trie), but in practice, it will contain only one element
// (since the view should be updated on every atomic change (XXX should it?) ).
//


// Changes to lists:
// - C# (NotifyCollectionChangedEventArgs)
//      - Add/Move/Remove/Replace/Reset
//      - List of added/removed items OR list of affected items + move indices
// - Java (FX) (javafx.collections.ListChangeListener)
//      - list of Changes (can contain both add and remove changes)
//      - change is: permutation(range), add(list + range), remove(idem), update(range)
//      - add/remove changes are sorted
// - C++/Qt (QAbstractItemModel)
//      - insert OR move OR remove rows + persistent indices


// in what cases does the position of an element in a sequence defines its identity?

// The trace should be a "summary of changes" since the last revision.
// -> has a method "focus" that returns a more precise trace if the data under the lens was modified,
//      None otherwise.
// -> e.g. node added, inputs/outputs inserted
//    nodes change (insert node id)
//    nodes[#x] change (partial)
//      -> collapse to just "node change"

// Start [a b c d]

// Add 0            [x a b c d]
// Remove 1         [x b c d]
// Move 2 to 1      [x c b d]
// Remove 1         [x b d]
// Add 0            [y x b d]

// Somehow collapse into a coherent diff from [a b c d] to [y x b d]
// (remove 0, remove 2, add 0..1)

// Diffing?
// -> remember that we can clone all the data (although it's not guaranteed to be memory efficient)
//
// Reconciliation?
//
// Principle:
// - in an ordered list where elements have no identity (identity-less values), any modification means a reset
//      - Change::Replace
// - in an ordered list where elements have identity, need to track inserted and removed element IDs
//      - rebuild the whole list
//      - Change::Elements { added, removed }
//
// Don't need to track the change precisely: let the view do the reconciliation
// - the view has the previous list of view element, each associated to an id
// - compare the current id list to the view ids, make intelligent diff
//
// Problem: list might be big, diff might be inefficient.
//
// Changes:
// - replace
// - collection:
//      - elements added    (stable IDs)
//      - elements removed  (stable IDs)
//      - elements updated  (stable IDs)
//      - relayout

//
// Problem: recording all changes
// - all changes are recorded in the revision list, but when to cleanup?
// - instead, register listeners/change trackers/whatever that track modifications to a part of the
//   model
//      - those change trackers must borrow the db (shared borrow)
//      - but: the db can provide scoped exclusive access to the db contents (wrap in refcell)

// what about an ordered list of layers?
// -> the identity of the layer is not its position in the stack
// (e.g. if you refer to a particular layer somewhere else, and then reorder layers, the reference
//  should still point to the same layer)


// persistent (immutable) data structures?
// - benefits:
//      - free undo/redo
//      - already implemented
//      - works well with lenses (lenses are made for mutating immutable data structures)
// - drawbacks:
//      - performance impact?
//      - overhead?
// - considerations:
//      - one of the reasons to use PDSs is to share stuff between threads (can mutate the data independently) without risk of data race.
//        but this is already prevented by rust
//          - the other reason is to keep the previous revisions of the state

// what are the benefits of the current approach?
// - the state might be stored in memory more efficiently
//      - no need to split collections into trees
// - less memory overhead: store
// However: to access a previous state, need to replay updates backwards (full copy)
//
// The thing is: for undo-redo, we don't need all previous versions to be available at the same time.
// also, PDSs were suggested only to solve the problem of step-by-step change tracking

/*
pub trait DatabaseSync {
    type Source: Model;

    fn apply_changes(&mut self, source: &Self::Source, changes: &[Change<Self::Source>]);

    fn sync(&mut self, from: &Database<Self::Source>) {
        self.apply_changes(&from.data, from.changes_since_revision())
    }
}
*/