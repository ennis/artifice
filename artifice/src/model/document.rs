use crate::model::{attribute::AttributeAny, ModelPath, Node, ShareGroup};
use imbl::Vector;
use kyute_common::Data;

/// Root object of documents.
#[derive(Clone, Debug)]
pub struct Document {
    /// Document revision index
    pub revision: usize,
    /// Root node
    pub root: Node,
    /// Share groups
    pub share_groups: Vector<ShareGroup>,
}

impl Data for Document {
    fn same(&self, other: &Self) -> bool {
        self.revision.same(&other.revision)
            && self.root.same(&other.root)
            && self.share_groups.ptr_eq(&other.share_groups)
    }
}

impl Document {
    /// Finds the node with the given path.
    pub fn find_node(&self, path: &ModelPath) -> Option<&Node> {
        match path.split_last() {
            None => Some(&self.root),
            Some((prefix, last)) => {
                let parent = self.find_node(&prefix)?;
                parent.find_child(&last)
            }
        }
    }

    /// Finds the node with the given path and returns a mutable reference to it.
    pub fn find_node_mut(&mut self, path: &ModelPath) -> Option<&mut Node> {
        match path.split_last() {
            None => Some(&mut self.root),
            Some((prefix, last)) => {
                let parent = self.find_node_mut(&prefix)?;
                parent.find_child_mut(&last)
            }
        }
    }

    /// Returns the attribute at the given path.
    pub fn find_attribute(&self, path: &ModelPath) -> Option<&AttributeAny> {
        assert!(path.is_attribute());
        let parent = path.parent().unwrap();
        let node = self.find_node(&parent)?;
        node.find_attribute(&path.name())
    }
}
