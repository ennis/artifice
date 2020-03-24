//! Stack of widget IDs.
use std::any::Any;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// The ID type.
pub type NodeId = u64;

/// The ID stack. Each level corresponds to a parent ItemNode.
struct IdStack(pub(super) Vec<NodeId>);

impl IdStack {
    /// Creates a new IdStack and push the specified ID onto it.
    pub fn new(root_id: NodeId) -> IdStack {
        IdStack(vec![root_id])
    }

    fn chain_hash<H: Hash>(&self, s: &H) -> NodeId {
        let stacklen = self.0.len();
        let key1 = if stacklen >= 2 {
            self.0[stacklen - 2]
        } else {
            0
        };
        let key0 = if stacklen >= 1 {
            self.0[stacklen - 1]
        } else {
            0
        };
        let mut hasher = DefaultHasher::new();
        key0.hash(&mut hasher);
        key1.hash(&mut hasher);
        s.hash(&mut hasher);
        hasher.finish()
    }

    /// Hashes the given data, initializing the hasher with the items currently on the stack.
    /// Pushes the result on the stack and returns it.
    /// This is used to generate a unique ID per item path in the hierarchy.
    pub fn push_id<H: Hash>(&mut self, s: &H) -> NodeId {
        let id = self.chain_hash(s);
        //let parent_id = *self.0.last().unwrap();
        self.0.push(id);
        id
    }

    /// Pops the ID at the top of the stack.
    pub fn pop_id(&mut self) {
        self.0.pop();
    }
}

pub struct StateCtx {
    stack: IdStack,
    state: HashMap<NodeId, Box<dyn Any>>,
}

impl StateCtx {
    pub(super) fn new() -> StateCtx {
        StateCtx {
            stack: IdStack::new(0),
            state: HashMap::new(),
        }
    }

    pub fn with_id<H: Hash, R, F: FnOnce(&mut StateCtx) -> R>(&mut self, s: &H, f: F) -> R {
        self.stack.push_id(s);
        let r = f(self);
        self.stack.pop_id();
        r
    }
}

// widget
// visual
// cache
//
// scene -> cached layout stuff + state
// stage -> where to render the scene, create using a
// painter ->
//
// stage:
//      - owns PlatformWindow,
//      - implements WindowHandler,
//           - receives events, translate them, and feed them to the UI, collect actions
//           - dispatch actions to handler
//      - provide an "application" trait that generates the widget tree
//          - own state, or reference application state through RC
//          -> long-lived "borrow"
//      - some widgets (context menu), when repainted, check if the window is already opened
// Child dialogs / popup windows:
//  - another WindowHandler
//  - send actions to parent window
//
