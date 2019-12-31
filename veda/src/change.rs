use std::ops::Range;

/// Summary of changes to a collection.
///
/// Represents a summary of the modifications applied to a collection between two predefined points
/// in time.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CollectionChanges {
    Relayout,
    Splice {
        start: usize,
        remove: usize,
        insert: usize,
    },
    Update {
        // not Range<usize> because Range<T> is not copy for reasons
        start: usize,
        end: usize,
    },
}

/*#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionChanges0 {
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
}*/

/// Describes a change applied to some data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Change {
    /// No changes
    None,
    /// Elements were inserted or removed into a collection.
    Collection(CollectionChanges),
    /// The entire element was replaced.
    Replace,
}

impl Change {
    pub fn replacement() -> Change {
        Change::Replace
    }

    pub fn empty() -> Change {
        Change::None
    }

    pub fn relayout() -> Change {
        Change::Collection(CollectionChanges::Relayout)
    }

    pub fn insertion(at: usize, len: usize) -> Change {
        Change::Collection(CollectionChanges::Splice {
            start: at,
            remove: 0,
            insert: len,
        })
    }

    pub fn deletion(at: usize, len: usize) -> Change {
        Change::Collection(CollectionChanges::Splice {
            start: at,
            remove: len,
            insert: 0,
        })
    }

    pub fn splicing(at: usize, remove: usize, insert: usize) -> Change {
        Change::Collection(CollectionChanges::Splice {
            start: at,
            remove,
            insert,
        })
    }

    pub fn update(range: Range<usize>) -> Change {
        Change::Collection(CollectionChanges::Update {
            start: range.start,
            end: range.end,
        })
    }
}
