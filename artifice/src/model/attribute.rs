use crate::{
    json,
    model::{Atom, Metadata, NamedObject, Value},
};
use anyhow::Error;
use imbl::{HashMap, Vector};
use kyute::Data;
use lazy_static::lazy_static;

////////////////////////////////////////////////////////////////////////////////////////////////////
// AttributeAny
////////////////////////////////////////////////////////////////////////////////////////////////////

/// An attribute of a `Node`.
#[derive(Clone, Debug, Data)]
pub struct AttributeAny {
    /// Base named object.
    pub base: NamedObject,

    /// Type identifier.
    pub ty: Atom,

    /// Value of the property.
    pub value: Option<Value>,

    /// Metadata
    pub metadata: HashMap<Atom, Value>,
}

impl AttributeAny {
    /// Name of the property
    pub fn name(&self) -> Atom {
        self.base.name()
    }

    /// Type ID of the property
    pub fn type_id(&self) -> &Atom {
        &self.ty
    }

    pub fn dump(&self, indent: usize) {
        println!("{:indent$}name  : {}", "", self.name, indent = indent);
        println!("{:indent$}type  : {}", "", self.ty, indent = indent);
        println!("{:indent$}value : {:?}", "", self.value, indent = indent);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// AttributeType
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Trait implemented by types that can be the type of a node attribute.
pub trait AttributeType {
    /// Returns the name of the type.
    fn name() -> Atom;
}

macro_rules! impl_attribute_type {
    ($t:ty, $id:ident, $atom:ident) => {
        lazy_static! {
            static ref $atom: Atom = Atom::new(std::stringify!($id));
        }

        impl AttributeType for $t {
            fn name() -> Atom {
                $atom.clone()
            }
        }
    };
}

impl_attribute_type!(f64, f64, ATTRIBUTE_TYPE_F64);
impl_attribute_type!(Atom, atom, ATTRIBUTE_TYPE_ATOM);
impl_attribute_type!(String, string, ATTRIBUTE_TYPE_STRING);

////////////////////////////////////////////////////////////////////////////////////////////////////
// ImagingEvalCtx
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Attribute<T> {}

/*#[derive(Clone, Debug, Data)]
pub struct TypedAttribute<T> {
    pub attribute: Attribute,
    pub value: T,
}*/
