use crate::model::{Atom, Metadata, Value};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct AttributeSchema {
    /// Name of the attribute.
    pub name: Atom,
    /// Expected type of the attribute.
    pub ty: Atom,
    /// Default value.
    pub default_value: Option<Value>,
    /// Default metadata.
    pub metadata: HashMap<Atom, Value>,
}

/// A set of required attributes on a node.
pub struct Schema {
    pub attributes: Vec<AttributeSchema>,
}
