use veda::db::{Append, Database, Replace, View, Change};
use veda::lens::{Lens, LensCompose, PartialPath};
use veda::lens::{LensExt, LensIndexExt};
use kyute::application::{Application, ensure_qt_initialized};
use kyute::miniqt_sys::*;
use std::marker::PhantomData;
use std::rc::Rc;
use veda::lens::path::PartialPathSlice;

#[derive(Lens, Clone, Debug)]
pub struct NodeInput {
    name: String,
}

#[derive(Lens, Clone, Debug)]
pub struct NodeOutput {
    name: String,
}

#[derive(Lens, Clone, Debug)]
pub struct Node {
    name: String,
    inputs: Vec<NodeInput>,
    outputs: Vec<NodeOutput>,
}

#[derive(Lens, Clone, Debug)]
pub struct Document {
    counter: i32,
    nodes: Vec<Node>,
    connections: Vec<(i32, i32)>,
}

/*
    enum Document {
        Counter,
        Nodes(Option<Vec::<Node>::Lens>),
        Connections(Option<Vec::<(i32,i32)>::Lens>),
    }

    // associated type "Address": (associated to partial lens)
    // 

*/

pub struct TestView;

impl View<Document> for TestView {
    fn on_change(&self, path: PartialPathSlice<Document>, change: &Change) {
        eprintln!("TestView: {:?} ({:?})", path, change);
        if let Some(path) = path.starts_with(Document::connections) {
            eprintln!("changed connections");
            // PathSlice::index<I: Index>(lens) -> Option<(I, PathSlice<...>)> where T:
        }
    }
}

// Lenses are not only "values", but also "patterns"
// -> like
// match path {
//      Document::connections::index(i) => ...,
// }
//
// -> ideally: pattern-match lenses
//

fn main() {
    let m = Document {
        counter: 0,
        nodes: Vec::new(),
        connections: Vec::new(),
    };

    let first_output_of_first_node = Document::nodes.index(0).compose(Node::inputs).index(0);
    // alternate macro-based syntax: lens!(<Document>.nodes[0].inputs[0])

    eprintln!(
        "first_output_of_first_node: {:?}",
        first_output_of_first_node.path().partial()
    );

    let mut db = Database::new(m);
    let view = Rc::new(TestView);
    db.add_view(view.clone());

    db.append(
        Document::nodes,
        Node {
            inputs: Vec::new(),
            outputs: Vec::new(),
            name: "hello".to_string(),
        },
    );

    db.append(
        Document::nodes.index(0).compose(Node::inputs),
        NodeInput { name: "input0".to_string() }
    );

    db.replace(
        Document::nodes.index(0).compose(Node::inputs).index(0).compose(NodeInput::name),
        "input1".to_string()
    );

    db.append(
        Document::connections,
        (0,1)
    );



    //dbg!(db.data());
}
