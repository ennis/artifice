//! Application data model
use crate::json;
use anyhow::Context;
use kyute::Data;
use lazy_static::lazy_static;
use imbl::Vector;
use std::{
    collections::HashMap,
    fs::File,
    path::Path,
    sync::{Arc, Weak},
};

pub mod atom;
mod document;
pub mod node;
mod path;
pub mod property;
mod share_group;
mod value;

pub use atom::Atom;
pub use document::{Document, DocumentModel, Edit};
pub use node::Node;
pub use path::ModelPath;
pub use value::Value;

/// Objects that belong to the document object tree, they have a name and are accessible via their path.
#[derive(Clone, Debug, Data)]
pub struct NamedObject {
    /// rowid in the `named_objects` table.
    pub id: i64,
    /// Path of this object in the document tree. Contains the name of the object.
    pub path: ModelPath,
}

impl NamedObject {
    /// Returns the name of this object.
    pub fn name(&self) -> Atom {
        self.path.name()
    }
}
