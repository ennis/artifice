//! Application data model
mod attribute;
mod document;
mod edit;
mod error;
mod file;
pub mod metadata;
mod named_object;
mod node;
mod path;
mod schema;
mod share_group;
mod value;

pub use attribute::AttributeAny;
pub use document::Document;
pub use edit::EditAction;
pub use error::Error;
pub use file::{DocumentDatabase, DocumentEditProxy, DocumentFile, NodeEditProxy};
pub use kyute_common::Atom;
pub use metadata::Metadata;
pub use node::Node;
pub use path::Path;
pub use share_group::ShareGroup;
pub use value::{FromValue, Value};

/// A dummy type for image-typed attributes.
///
/// It doesn't hold a value because it's not serializable.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Image;
