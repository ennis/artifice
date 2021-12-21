use crate::model::node::Node;
use std::sync::Arc;

/// A hierarchical set of interconnected nodes.
#[derive(Clone, Debug, Data)]
pub struct Network {
    pub root: Arc<Node>,
}

impl Network {
    /// Creates a new, empty network.
    pub fn new() -> Network {
        let root = Node::new();
        Network {
            root: TreeNodeData::new(root),
        }
    }

    /// Returns the root node
    pub fn root_node(&self) -> &Node {
        &self.root.node
    }

    /// Returns a lens over the tree root (root node + selection)
    pub fn tree_root_lens() -> impl Lens<Network, TreeNodeData<Node>> {
        druid::lens!(Network, root)
    }

    /// Serializes this node to JSON.
    pub fn to_json(&self) -> serde_json::Value {
        self.root.node.to_json()
    }

    /// Creates a network from a JSON description.
    pub fn from_json(value: &serde_json::Value) -> anyhow::Result<Network> {
        let root_node = Node::from_json(value)?;
        Ok(Network {
            root: TreeNodeData::new(root_node),
        })
    }
}
