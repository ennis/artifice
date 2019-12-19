use std::fmt;
use std::marker::PhantomData;
use crate::db::Data;
use crate::lens::Lens;


#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ComponentIndex(u64);

impl fmt::Debug for ComponentIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, ".{}", self.0)
    }
}

// problem: a lens can access a vector in different ways
// - by element key
// - by index
// this means that `.0` is different from `.k#0`
// However, it makes no sense to allow both `.f#0` (access by field) and `.0` or `.k#0`
//
// Proposal: disallow access by index and element key on the same type
// -> type of access (type of indexing) is specified by type
// For Vec with linear search, provide a wrapper. (e.g. EntityVec or something)
//
// A path component becomes just a u64, the interpretation of which depends on the Root type associated
// with the path.

//
// Issue: this assumes that every path is representable by a sequence of u64
// but what about other key types?
// -> strings: use u64 tokens instead
// -> domain-specific data, for instance, a geographical location
//      -> a geographical location does not define identity of an object

//
// bikeshedding:
// - name of the library for the lenses and database-like access (state management)
//      - wraps state and provides a way to track, record and undo changes to the state
//      - database-something / or " data store"
//      - "document model"
//      - single-source-of-truth for the whole application
//      - * veda

// - name of the elements of a lens path
//      - key
//      - coordinate (confusion possible with spatial coordinates)
//      - index (confusion possible with linear index)
//      - * id (preferred)

// PartialPath<Root>
//  -> PartialPathRef<'a, Root>
//      -> (first component) -> PathComponent<'a, Root

/// Slice of a lens path.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct PartialPathSlice<'a, Root: Data>(&'a [ComponentIndex], PhantomData<*const Root>);

impl<'a, Root: Data> fmt::Debug for PartialPathSlice<'a, Root> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for c in self.0.iter() {
            c.fmt(f)?
        }
        Ok(())
    }
}

impl<'a, Root: Data> PartialPathSlice<'a, Root> {
    /// Checks if the path starts with the path of the given lens.
    ///
    /// Returns the remainder of the path if that's the case, or None if the start of the path does
    /// not match the given lens.
    pub fn starts_with<L: Lens<Root=Root>>(&self, lens: L) -> Option<PartialPathSlice<L::Leaf>> {
        let is = (lens.path().0).0;
        if self.0.starts_with(&is[..]) {
            Some(PartialPathSlice(&self.0[is.len()..], PhantomData))
        } else {
            None
        }
    }

    /// Returns the first component index.
    pub fn first_index(&self) -> Option<ComponentIndex> {
        self.0.first().cloned()
    }

    /// Returns whether the path is empty.
    ///
    /// If it is, then this is the path of the identity lens for `Root`.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn indices(&self) -> &'a [ComponentIndex] {
        self.0
    }
}

/// Represents a path to a part of an object.
///
/// Given some type T and a `Lens` over T, a `LensPath` uniquely identifies the sub-part of the type
/// T that the lens is watching.
///
/// For example, if you have a `Lens<Vec<U>, U>` that returns the element at index `i` in the `Vec`,
/// then the `LensPath` contains the value of `i`. For a lens that views a field of a struct type,
/// the `LensPath` is an unique identifier for the field (for instance, its index in field
/// declaration order).
///
/// For lenses that access deeper parts of an object, the corresponding `LensPath` is obtained
/// by concatenating the `LensPath`s of each primitive indexing and field access operations that
/// make up the lens.
///
/// Note that the comparison of `LensPath`s only makes sense in the context of some common source
/// type T. Notably, given `t: Lens<T, _>` and `u:Lens<U, _>`, it is possible that `t.path() == u.path()`
/// even if T and U are completely unrelated.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct PartialPath<Root: Data>(Vec<ComponentIndex>, PhantomData<*const Root>);

impl<Root:Data> PartialPath<Root> {
    pub fn as_slice(&self) -> PartialPathSlice<Root> {
        PartialPathSlice(&self.0[..], PhantomData)
    }
    pub fn indices(&self) -> &[ComponentIndex] { &self.0[..] }

    pub fn into_indices(self) -> Vec<ComponentIndex> {
        self.0
    }
}

impl<Root: Data> fmt::Debug for PartialPath<Root> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.as_slice().fmt(f)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Path<Root: Data, Leaf: Data>(PartialPath<Root>, PhantomData<*const Leaf>);

impl<Root: Data, Leaf: Data> From<Path<Root, Leaf>> for PartialPath<Root> {
    fn from(p: Path<Root, Leaf>) -> Self {
        p.0
    }
}

impl<Root: Data, Leaf: Data> Path<Root, Leaf> {
    pub fn partial(&self) -> &PartialPath<Root> {
        &self.0
    }

    pub fn indices(&self) -> &[ComponentIndex] { self.0.indices() }
    pub fn into_indices(self) -> Vec<ComponentIndex> { self.0.into_indices() }

    pub fn append<NewLeaf: Data>(mut self, mut other: Path<Leaf, NewLeaf>) -> Path<Root, NewLeaf> {
        (self.0).0.append(&mut (other.0).0);
        Path(self.0, PhantomData)
    }

    pub fn index(i: usize) -> Path<Root, Leaf> {
        Path::from_components(vec![ComponentIndex(i as u64)])
    }

    pub fn field(field_index: usize) -> Path<Root, Leaf> {
        Path::from_components(vec![ComponentIndex(field_index as u64)])
    }

    pub fn identity() -> Path<Root, Leaf> {
        Path::from_components(vec![])
    }

    pub fn key(k: u64) -> Path<Root,Leaf> {
        Path::from_components(vec![ComponentIndex(k)])
    }

    fn from_components(c: Vec<ComponentIndex>) -> Path<Root,Leaf> {
        Path(PartialPath(c, PhantomData), PhantomData)
    }
}
