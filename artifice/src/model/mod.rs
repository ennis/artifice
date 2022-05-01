//! Application data model
use crate::json;
use anyhow::Context;
use imbl::Vector;
use kyute::Data;
use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    fs::File,
    path::Path,
    sync::{Arc, Weak},
};

mod attribute;
mod connection;
mod document;
pub mod metadata;
mod named_object;
mod node;
mod path;
mod schema;
mod share_group;
mod value;

pub use attribute::Attribute;
pub use connection::{DocumentConnection, Edit};
pub use document::Document;
pub use kyute_common::Atom;
pub use metadata::Metadata;
pub use named_object::NamedObject;
pub use node::Node;
pub use path::ModelPath;
pub use share_group::ShareGroup;
pub use value::{FromValue, Value};

/// A dummy type for image-typed attributes.
///
/// It doesn't hold a value because it's not serializable.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Image;
