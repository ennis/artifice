use crate::db::Data;
use std::fmt::Debug;
use std::marker::PhantomData;

pub use veda_macros::Lens;

pub mod path;
pub mod vec;

pub use path::ComponentIndex;
pub use path::PartialPath;
pub use path::Path;
use std::hash::Hash;

/// terminology:
///
/// # Object
/// an object
///
/// # Aggregate
/// An object constituted of one or more sub-objects. This can be a structure, a collection, etc.
///
/// # Lens
/// A lens is an object that represents a way to access a component of some complex aggregate type `U`.
///
/// Concretely, a lens over a type `U`, given a reference to an aggregate of type `U`,
/// provides access to an object of type `V` stored within the aggregate
/// (and potentially deep within the structure of `U`).
/// `U` is called the _root type_ and `V` is called the _leaf type_.
/// For all intents and purposes, you can see lenses as a generic way to represent a sequence of
/// field accesses, indexing operations, and lookups that access an object
/// stored arbitrarily deep into an object of type `U`.
/// (e.g. `.field.collection[42].field2.map[21]`).
///
/// The _lens_ term is borrowed from the concept of _functional lenses_ in some languages
/// (https://www.schoolofhaskell.com/school/to-infinity-and-beyond/pick-of-the-week/basic-lensing).
/// A possible synonym of lens could be "accessor".
///
///
/// # Lens path
/// A value that uniquely identifies a lens over a type `U`. If two lenses `K` and `L` over the same
/// type `U` have equal paths, then they represent access to the same component within `U`.
/// Lens paths can be decomposed into a sequence of _component indices_, with each index representing one
/// primitive component access operation (field access, indexing operation, or lookup).
///
/// Component indices are u64 integers. Depending on the type of object that the component index
/// applies to, the index can represent either a structure field, an index in a linear collection,
/// or a key to access an element in an associative container.
///


//---------------------------------------------------------------------------

/// Trait implemented by "lens" types, which act like a reified accessor for
/// some part of type U of an object of type T.
pub trait Lens: Clone + 'static {
    type Root: Data;
    type Leaf: Data;

    /// Returns a value that represents the path from the object to the part
    /// that the lens is watching (i.e. the path from &T to &U)
    fn path(&self) -> path::Path<Self::Root, Self::Leaf>;

    /// Returns a reference to the object part that the lens is watching.
    fn get<'a>(&self, data: &'a Self::Root) -> &'a Self::Leaf;

    /// Returns a mutable reference to the object part that the lens is watching.
    fn get_mut<'a>(&self, data: &'a mut Self::Root) -> &'a mut Self::Leaf;

    /// TODO
    fn try_get<'a>(&self, data: &'a Self::Root) -> Option<&'a Self::Leaf>;

    /// TODO
    fn try_get_mut<'a>(&self, data: &'a mut Self::Root) -> Option<&'a mut Self::Leaf>;
}

/// Indexing operations
pub trait LensIndexExt: Lens {
    type Output: Lens;
    fn index(&self, i: usize) -> Self::Output;
}

pub trait EntityKey: Copy + Clone + Debug + Eq + PartialEq + Hash {
    fn to_u64(&self) -> u64;
}

/// Entities
pub trait Entity: Data {
    type Key: EntityKey;
    fn key(&self) -> Self::Key;
}

/// Key lookup operations
pub trait LensLookupExt: Lens {
    type Key: Copy + Clone + Debug + Eq + PartialEq + Hash;
    type Output: Lens;
    fn by_key(&self, key: Self::Key) -> Self::Output;
}

/// Composition
pub trait LensExt: Lens {
    fn compose<K>(&self, other: K) -> LensCompose<Self, K>
    where
        K: Lens<Root = <Self as Lens>::Leaf>,
    {
        LensCompose(self.clone(), other)
    }
}

impl<L: Lens> LensExt for L {}


/// Identity lens.
#[derive(Clone, Debug)]
pub struct IdentityLens<U: Data>(PhantomData<*const U>);

impl<U: Data> IdentityLens<U> {
    pub fn new() -> IdentityLens<U> {
        IdentityLens(PhantomData)
    }
}

impl<U: Data> Lens for IdentityLens<U> {
    type Root = U;
    type Leaf = U;

    fn path(&self) -> Path<U, U> {
        Path::identity()
    }

    fn get<'a>(&self, data: &'a U) -> &'a U {
        data
    }

    fn get_mut<'a>(&self, data: &'a mut U) -> &'a mut U {
        data
    }

    fn try_get<'a>(&self, data: &'a Self::Root) -> Option<&'a Self::Leaf> {
        Some(data)
    }

    fn try_get_mut<'a>(&self, data: &'a mut Self::Root) -> Option<&'a mut Self::Root> {
        Some(data)
    }
}


/// Lens composition: combines `Lens<U,V>` and `Lens<V,W>` to `Lens<U,W>`.
///
/// Equivalent to applying two lenses in succession.
#[derive(Clone, Debug)]
pub struct LensCompose<L1, L2>(pub L1, pub L2);

impl<L1, L2> Lens for LensCompose<L1, L2>
    where
        L1: Lens,
        L2: Lens<Root = L1::Leaf>,
{
    type Root = L1::Root;
    type Leaf = L2::Leaf;

    fn path(&self) -> Path<Self::Root, Self::Leaf> {
        self.0.path().append(self.1.path())
    }

    fn get<'a>(&self, data: &'a Self::Root) -> &'a Self::Leaf {
        self.1.get(self.0.get(data))
    }

    fn get_mut<'a>(&self, data: &'a mut Self::Root) -> &'a mut Self::Leaf {
        self.1.get_mut(self.0.get_mut(data))
    }

    fn try_get<'a>(&self, data: &'a Self::Root) -> Option<&'a Self::Leaf> {
        self.1.try_get(self.0.try_get(data)?)
    }

    fn try_get_mut<'a>(&self, data: &'a mut Self::Root) -> Option<&'a mut Self::Leaf> {
        self.1.try_get_mut(self.0.try_get_mut(data)?)
    }
}


/*
/// Macro for implementing a lens type that accesses a field of a struct.
#[macro_export]
macro_rules! impl_field_lens {
    ($v:vis $lens:ident [ $t:ty => $u:ty ] [ $f:ident ( $index:expr ) ]) => {
        #[derive(Copy,Clone,Debug)]
        $v struct $lens;
        impl $crate::lens::Lens for $lens {
            type Root = $t;
            type Leaf = $u;

            fn path(&self) -> $crate::lens::Path<$t, $u> {
                $crate::lens::Path::field($index)
            }

            fn get<'a>(&self, data: &'a $t) -> &'a $u {
                &data.$f
            }

            fn get_mut<'a>(&self, data: &'a mut $t) -> &'a mut $u {
                &mut data.$f
            }
        }
    };
}
*/