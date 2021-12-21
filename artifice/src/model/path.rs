use crate::model::atom::Atom;
use dashmap::DashMap;
use lazy_static::lazy_static;
use std::sync::Arc;
use kyute::Data;

enum PathNodeKind {
    Root,
    Part { name: Atom },
}

// TODO they should probably be allocated in an object pool
struct PathNode {
    parent: Option<Arc<PathNode>>,
    kind: PathNodeKind,
}

impl PathNode {
    fn new_root() -> Arc<PathNode> {
        Arc::new(PathNode {
            parent: None,
            kind: PathNodeKind::Root,
        })
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct PathKey {
    parent: *const PathNode,
    part: Atom,
}

lazy_static! {
    static ref ROOT_PATH_NODE: Arc<PathNode> = PathNode::new_root();
    static ref PATH_NODE_TABLE: DashMap<PathKey, Arc<PathNode>> = DashMap::new();
}

fn root_path_node() -> &'static Arc<PathNode> {
    &*ROOT_PATH
}

fn path_node_table() -> &'static DashMap<PathKey, Arc<PathNode>> {
    &*PATH_TABLE
}

//--------------------------------------------------------------------------------------------------

/// Paths of the form:
///
/// # Examples of paths
///
/// - `/network/node/param`: absolute path
///

// `/network/node/param`: 4 nodes in the chain.
#[derive(Clone, Debug, Data)]
pub struct ModelPath {
    node: Arc<PathNode>,
}

impl ModelPath {
    /// Returns the path to the root object.
    pub fn root() -> ModelPath {
        ModelPath {
            node: ROOT_PATH_NODE.clone(),
        }
    }

    /// Concatenates.
    pub fn join(&self, part: Atom) -> ModelPath {
        let node = PATH_NODE_TABLE
            .entry(PathKey {
                parent: Arc::as_ptr(&self.node),
                part: part.clone(),
            })
            .or_insert_with(|| {
                Arc::new(PathNode {
                    parent: Some(self.node.clone()),
                    kind: PathNodeKind::Root,
                })
            });

        ModelPath { node }
    }

    /// Relative path.
    pub fn is_relative(&self) -> bool {
        false
    }

    /// Whether this is an absolute path.
    pub fn is_absolute(&self) -> bool {
        true
    }

    /// Whether this is the root path.
    pub fn is_root(&self) -> bool {
        Arc::as_ptr(&self.node) == Arc::as_ptr(&*ROOT_PATH_NODE)
    }

    /// Returns the parent path.
    pub fn parent(&self) -> Option<ModelPath> {
        self.node.parent.clone().map(|node| ModelPath { node })
    }

}
