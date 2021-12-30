//! Application data model
use crate::{
    json,
    model::{atom::Atom, network::Network, node::Node, path::ModelPath, value::Value},
};
use anyhow::Context;
use kyute::Data;
use lazy_static::lazy_static;
use std::{collections::HashMap, fs::File, path::Path, sync::Arc};
use std::sync::Weak;
use rpds::Vector;
use crate::model::document::DocumentModel;

pub mod atom;
mod document;
pub mod node;
mod path;
pub mod property;
mod share_group;
mod value;

/// Objects that belong to the document object tree, they have a name and are accessible via their path.
#[derive(Clone, Debug)]
pub struct NamedObject {
    /// Parent document (or null if orphaned).
    pub document: Weak<DocumentModel>,
    /// rowid in the `named_objects` table.
    pub id: i64,
    /// Path of this object in the document tree. Contains the name of the object.
    pub path: ModelPath,
}

impl Data for NamedObject {
    fn same(&self, other: &Self) -> bool {
        self.0.same(&other.0)
    }
}

impl NamedObject {
    /// Returns the name of this object.
    pub fn name(&self) -> Atom {
        self.path.name()
    }
}
