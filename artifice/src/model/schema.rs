use crate::model::{Metadata, Value};
use kyute_common::Atom;

#[derive(Clone, Debug)]
pub struct AttributeSchema {
    /// Name of the attribute.
    pub name: Atom,
    /// Expected type of the attribute.
    pub ty: Atom,
    /// Default value.
    pub default_value: Option<Value>,
    /// Default metadata.
    pub metadata: Vec<Metadata>,
}

/// A set of required attributes on a node.
pub struct Schema {
    pub attributes: Vec<AttributeSchema>,
}
