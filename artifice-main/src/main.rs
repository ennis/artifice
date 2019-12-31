#![feature(specialization)]
use kyute::view::{ ViewExt};
use veda::lens::LensIndexExt;
use veda::{Data, Database, Identifiable, Lens};

#[derive(Data, Clone, Debug)]
pub struct NodeInput {
    name: String,
}

impl Identifiable for NodeInput {
    type Id = String;

    fn id(&self) -> String {
        self.name.clone()
    }
}

#[derive(Data, Clone, Debug)]
pub struct NodeOutput {
    name: String,
}

#[derive(Data, Clone, Debug)]
pub struct Node {
    name: String,
    inputs: Vec<NodeInput>,
    outputs: Vec<NodeOutput>,
}

#[derive(Data, Clone, Debug)]
pub struct Document {
    counter: i32,
    nodes: Vec<Node>,
    connections: Vec<(i32, i32)>,
}

/*pub struct TestView;

impl View<Document> for TestView {
    fn on_change(&self, revision: Revision<Document>) {
        if let Some(rev) = revision.focus(Document::nodes) {
            eprintln!("changed nodes")
        } else if let Some(rev) = revision.focus(Document::connections) {
            eprintln!("changed connections")
        } else {
            eprintln!("changed something else")
        }
    }
}*/

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
    // alternate macro-based syntax: lens!(<Document>.nodes[0].inputs[0]) => Document::Nodes((0, Some(Node::Inputs(0, None))))

    let mut db = Database::new(m);
    //let view = Rc::new(TestView);
    //db.add_view(view.clone());

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
        NodeInput {
            name: "input0".to_string(),
        },
    );

    db.replace(
        Document::nodes
            .index(0)
            .compose(Node::inputs)
            .index(0)
            .compose(NodeInput::name),
        "input1".to_string(),
    );

    db.append(Document::connections, (0, 1));

    use kyute::view as kyv;

    #[derive(Clone, Debug)]
    enum Action {
        Clicked(String),
    }

    let root = kyv::Root::<Document, Action>::new(kyv::Lensed::new(
        Document::nodes.index(0),
        kyv::VBox::new(vec![
            Box::new(kyv::Lensed::new(Node::name, kyv::Label::new())),
            Box::new(kyv::Lensed::new(
                Node::inputs,
                kyv::List::new(|id: String| {
                    let id = id.clone();
                    Box::new(kyv::VBox::new(vec![
                        Box::new(kyv::Lensed::new(
                            NodeInput::name,
                            kyv::Button::new("hello").map(move |_| Action::Clicked(id.clone())),
                        )),
                        Box::new(kyv::Lensed::new(NodeInput::name, kyv::Label::new())),
                    ]))
                }),
            )),
        ]),
    ));

    db.add_watcher(root.clone());

    db.append(
        Document::nodes.index(0).compose(Node::inputs),
        NodeInput {
            name: "input2".to_string(),
        },
    );

    while !root.exited() {
        root.run();
    }

    eprintln!("exiting");

    //dbg!(db.data());
}
