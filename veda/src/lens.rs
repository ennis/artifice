use std::fmt::Debug;
use std::marker::PhantomData;

use crate::data::Data;
use std::hash::Hash;

pub mod vec;

pub use vec::VecAddress;
pub use vec::VecLens;

/// Trait implemented by "lens" types, which act like a reified accessor for
/// some part of type U of an object of type T.
pub trait Lens: Clone + 'static {
    type Root: Data;
    type Leaf: Data;

    /// Returns a value that represents the path from the object to the part
    /// that the lens is watching (i.e. the path from &T to &U)
    fn address(&self) -> Option<<Self::Root as Data>::Address>;

    /// Concatenate addresses.
    fn concat<L, U>(&self, rhs: L) -> Option<<Self::Root as Data>::Address>
    where
        L: Lens<Root = Self::Leaf, Leaf = U>,
        U: Data;

    /// Returns a reference to the object part that the lens is watching.
    fn get<'a>(&self, data: &'a Self::Root) -> &'a Self::Leaf;

    /// Returns a mutable reference to the object part that the lens is watching.
    fn get_mut<'a>(&self, data: &'a mut Self::Root) -> &'a mut Self::Leaf;

    /// TODO
    fn try_get<'a>(&self, data: &'a Self::Root) -> Option<&'a Self::Leaf>;

    /// TODO
    fn try_get_mut<'a>(&self, data: &'a mut Self::Root) -> Option<&'a mut Self::Leaf>;

    fn compose<K>(&self, rhs: K) -> LensCompose<Self, K>
    where
        K: Lens<Root = Self::Leaf>,
    {
        LensCompose(self.clone(), rhs)
    }

    fn unprefix(
        &self,
        addr: <Self::Root as Data>::Address,
    ) -> Option<Option<<Self::Leaf as Data>::Address>>;
}

/// Indexing operations
pub trait LensIndexExt: Lens {
    type Output: Lens;
    fn index(&self, i: usize) -> Self::Output;
}

/// Key lookup operations
pub trait LensLookupExt: Lens {
    type Key: Copy + Clone + Debug + Eq + PartialEq + Hash;
    type Output: Lens;
    fn by_key(&self, key: Self::Key) -> Self::Output;
}

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

    fn address(&self) -> Option<U::Address> {
        None
    }

    fn concat<L, T>(&self, rhs: L) -> Option<U::Address>
    where
        L: Lens<Root = U, Leaf = T>,
        T: Data,
    {
        rhs.address()
    }

    fn unprefix(&self, addr: U::Address) -> Option<Option<U::Address>> {
        Some(Some(addr))
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

    fn address(&self) -> Option<<L1::Root as Data>::Address> {
        self.0.concat(self.1.clone())
    }

    fn concat<L, T>(&self, rhs: L) -> Option<<L1::Root as Data>::Address>
    where
        L: Lens<Root = L2::Leaf, Leaf = T>,
        T: Data,
    {
        // XXX what's the complexity of this?
        self.0.concat(LensCompose(self.1.clone(), rhs.clone()))
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

    fn compose<K>(&self, rhs: K) -> LensCompose<Self, K>
    where
        K: Lens<Root = Self::Leaf>,
    {
        LensCompose(self.clone(), rhs)
    }

    fn unprefix(
        &self,
        addr: <L1::Root as Data>::Address,
    ) -> Option<Option<<L2::Leaf as Data>::Address>> {
        self.0
            .unprefix(addr)
            .and_then(|addr| addr.and_then(|addr| self.1.unprefix(addr)))
    }
}


/// Unit lens: returns () always
#[derive(Copy,Clone,Debug)]
pub struct UnitLens<T>(PhantomData<T>);

impl<T> UnitLens<T> {
    pub fn new() -> UnitLens<T> {
        UnitLens(PhantomData)
    }
}

impl<T: Data> Lens for UnitLens<T> {
    type Root = T;
    type Leaf = ();

    fn address(&self) -> Option<T::Address> {
        // No address within T (although there should be one, just not a meaningful one)
        None
    }

    fn concat<L, U>(&self, rhs: L) -> Option<T::Address> where
        L: Lens<Root=(), Leaf=U>,
        U: Data
    {
        None
    }

    fn get<'a>(&self, data: &'a Self::Root) -> &'a () {
        static V: () = ();
        &V
    }

    fn get_mut<'a>(&self, data: &'a mut Self::Root) -> &'a mut () {
        static mut V: () = ();
        // TODO is it safe? (shared mut ref to a ZST)
        // it works for array refs (special-cased in rustc for &mut []),
        // but apparently not for custom ZSTs
        // See also: https://rust-lang.github.io/rfcs/1414-rvalue_static_promotion.html
        unsafe { &mut V }
    }

    fn try_get<'a>(&self, data: &'a Self::Root) -> Option<&'a ()> {
        Some(self.get(data))
    }

    fn try_get_mut<'a>(&self, data: &'a mut Self::Root) -> Option<&'a mut ()> {
        Some(self.get_mut(data))
    }

    fn unprefix(&self, addr: T::Address) -> Option<Option<<() as Data>::Address>> {
        // never unprefixed by anything
        None
    }
}

/*
/// Composition
pub trait LensExt: Lens {
    fn compose<K>(&self, other: K) -> LensCompose<Self, K>
    where
        K: Lens<Root = <Self as Lens>::Leaf>,
    {
        LensCompose(self.clone(), other)
    }
}

impl<L: Lens> LensExt for L {}*/

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
