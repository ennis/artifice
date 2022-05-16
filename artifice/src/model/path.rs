use crate::model::{Atom, Error};
use dashmap::DashMap;
use kyute::Data;
use lazy_static::lazy_static;
use std::{
    convert::TryFrom,
    fmt,
    hash::{Hash, Hasher},
    sync::Arc,
};

#[derive(Clone, Debug)]
enum PathNodeKind {
    Root,
    Node { parent: Arc<PathNode>, name: Atom },
    Attribute { parent: Arc<PathNode>, name: Atom },
}

// TODO they should probably be allocated in an object pool
#[derive(Debug)]
struct PathNode {
    kind: PathNodeKind,
}

impl PathNode {
    fn new_root() -> Arc<PathNode> {
        Arc::new(PathNode {
            kind: PathNodeKind::Root,
        })
    }

    /// Converts the path represented by this node into its string representation.
    fn to_string(&self) -> String {
        match &self.kind {
            PathNodeKind::Root => "".to_string(),
            PathNodeKind::Node { parent, name } => {
                let mut p = parent.to_string();
                p.push_str("/");
                p.push_str(name.as_ref());
                p
            }
            PathNodeKind::Attribute { parent, name } => {
                let mut p = parent.to_string();
                p.push_str(".");
                p.push_str(name.as_ref());
                p
            }
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
struct PathKey {
    parent: usize, // pointer
    part: Atom,
}

lazy_static! {
    static ref ROOT_PATH_NODE: Arc<PathNode> = PathNode::new_root();
    static ref PATH_NODE_TABLE: DashMap<PathKey, Arc<PathNode>> = DashMap::new();
}

//--------------------------------------------------------------------------------------------------

/// Paths of the form:
///
/// # Examples of paths
///
/// - `/network/node/param`: absolute path
///
#[derive(Clone)]
pub struct Path {
    node: Arc<PathNode>,
}

impl fmt::Debug for Path {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "`{}`", self.to_string())
    }
}

impl From<Arc<PathNode>> for Path {
    fn from(node: Arc<PathNode>) -> Self {
        Path { node }
    }
}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        Arc::as_ptr(&self.node) == Arc::as_ptr(&other.node)
    }
}

impl Eq for Path {}

impl Hash for Path {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(Arc::as_ptr(&self.node) as usize)
    }
}

impl Data for Path {
    fn same(&self, other: &Self) -> bool {
        self.node.same(&other.node)
    }
}

fn is_valid_path_part(part: &str) -> bool {
    !part.contains(&['/', '.'])
}

impl Path {
    /// Returns the path to the root object.
    pub fn root() -> Path {
        Path {
            node: ROOT_PATH_NODE.clone(),
        }
    }

    fn insert_path_node(&self, part: Atom, is_attribute: bool) -> Arc<PathNode> {
        assert!(is_valid_path_part(&part));

        PATH_NODE_TABLE
            .entry(PathKey {
                parent: Arc::as_ptr(&self.node) as usize,
                part: part.clone(),
            })
            .or_insert_with(|| {
                Arc::new(PathNode {
                    kind: if is_attribute {
                        PathNodeKind::Attribute {
                            parent: self.node.clone(),
                            name: part.clone(),
                        }
                    } else {
                        PathNodeKind::Node {
                            parent: self.node.clone(),
                            name: part.clone(),
                        }
                    },
                })
            })
            .clone()
    }

    /// Returns a new path with the specified part appended to it.
    pub fn join(&self, part: impl Into<Atom>) -> Path {
        let node = self.insert_path_node(part.into(), false);
        Path { node }
    }

    pub fn join_attribute(&self, attrib: impl Into<Atom>) -> Path {
        assert!(
            matches!(self.node.kind, PathNodeKind::Node { .. }),
            "path should be a path to a node"
        );
        let node = self.insert_path_node(attrib.into(), true);
        Path { node }
    }

    /// Returns whether this path is relative.
    pub fn is_relative(&self) -> bool {
        false
    }

    /// Returns whether this is an absolute path.
    pub fn is_absolute(&self) -> bool {
        true
    }

    /// Returns whether this is a path to a node.
    ///
    /// # Example
    ///
    ///```rust
    /// assert!(ModelPath::parse("/node").unwrap().is_node());
    /// assert!(ModelPath::parse("/node/child").unwrap().is_node());
    /// assert!(!ModelPath::parse("/node.attr").unwrap().is_node());    // attribute path
    /// assert!(!ModelPath::parse("/").unwrap().is_node()); // root path is not considered a node
    ///```
    ///
    pub fn is_node(&self) -> bool {
        matches!(self.node.kind, PathNodeKind::Node { .. })
    }

    /// Returns whether this is a path to a node attribute.
    ///
    /// # Example
    ///
    /// TODO
    ///
    pub fn is_attribute(&self) -> bool {
        matches!(self.node.kind, PathNodeKind::Attribute { .. })
    }

    /// Returns whether this is the root path.
    ///
    /// # Example
    ///
    /// TODO
    ///
    pub fn is_root(&self) -> bool {
        Arc::as_ptr(&self.node) == Arc::as_ptr(&*ROOT_PATH_NODE)
    }

    /// Returns the parent path.
    pub fn parent(&self) -> Option<Path> {
        match self.node.kind {
            PathNodeKind::Root => None,
            PathNodeKind::Node { ref parent, .. } | PathNodeKind::Attribute { ref parent, .. } => {
                Some(Path { node: parent.clone() })
            }
        }
    }

    /// Splits the last part of the path, if there's one.
    ///
    /// Returns `None` for the root path.
    pub fn split_last(&self) -> Option<(Path, Atom)> {
        match self.node.kind {
            PathNodeKind::Root => None,
            PathNodeKind::Node { ref parent, ref name } | PathNodeKind::Attribute { ref parent, ref name } => {
                Some((Path { node: parent.clone() }, name.clone()))
            }
        }
    }

    /// Returns whether the specified path is a prefix of this one.
    pub fn is_prefix(&self, other: &Path) -> bool {
        let mut p = Some(other.clone());
        while let Some(pp) = p {
            if &pp == self {
                return true;
            }
            p = pp.parent();
        }
        false
    }

    /// Returns the name of the object referred to by the pass, which is the last part of the path.
    pub fn name(&self) -> Atom {
        match self.node.kind {
            PathNodeKind::Root => Atom::default(),
            PathNodeKind::Node { ref name, .. } | PathNodeKind::Attribute { ref name, .. } => name.clone(),
        }
    }

    /// Converts this path into a string representation.
    pub fn to_string(&self) -> String {
        match &self.node.kind {
            PathNodeKind::Root => "/".to_string(),
            _ => self.node.to_string(),
        }
    }

    /// Parses a path from a string representation.
    pub fn parse(path: &str) -> Option<Path> {
        // NOTE: right now parsing never fails, but it might in the future as path syntax evolves
        if let Some(pos) = path.rfind(&['/', '.']) {
            let prefix = &path[0..pos];
            let name = &path[pos + 1..];
            match path.as_bytes()[pos] {
                b'/' => Some(Self::parse(prefix)?.join(name)),
                b'.' => Some(Self::parse(prefix)?.join_attribute(name)),
                _ => unreachable!(),
            }
        } else {
            Some(Self::root())
        }
    }
}

impl<'a> TryFrom<&'a str> for Path {
    type Error = Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Path::parse(value).ok_or(Error::PathSyntax)
    }
}
