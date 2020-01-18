#![feature(specialization)]
//use kyute::view::{ ViewExt};
use kyute::view::ButtonAction;
use kyute::view::Property;
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

    let root = kyv::Root::new(kyv::VBox::new((
        kyv::Label::new(),
        kyv::VBox::new((
            {
                let mut label = kyv::Label::new();
                label.text().set("hello".into());
                label
            },
            kyv::Button::new("hello"),
            kyv::Button::new("world"),
        )),
    )));

    view! {
        pub view ViewName { // struct-like declaration
            in node: Node,          // input (bindable) property
            in label: String,
            state: i32,             // local state
        } ->  // (alternate anonymous prop: pub view ViewName(Type) -> ...)
        VBox {
            Label(.text <- label),
            VBox {
                Label(
                    [pattern = Node::name in node] .text = pattern;
                    .text <- Node::name in node;    // alternate short form of the above
                    .text <- node.name;     // ideal form, but how to get the lens for name?

                    // V1:
                    // - no local state
                    // - single input property (multiple properties supported in body)

                    // [... in ...] is an update guard, will update the value of the prop only if .name changed;
                    // n is the value of the prop (optional, can use node.name).
                    // issue: the guard needs to identify which prop it is watching
                    // the branch under a guard has access to the watched prop
                    // TODO: multiple update guards?
                    //
                    // problem: naming the lens
                    // - ideally: node.name
                    //      - but:

                    // can't access another property in a guarded declaration or property binding
                    // - because in update, we only see the property that has changed!
                    // OR: update() takes a diff of ALL properties at once, diff can be "unchanged"
                    // also: cannot double-guard (except if the prop is cached)

                    // Options:
                    // - A: Cache property values so they can be accessed at any time
                    //      - potentially expensive clone
                    // - B: A guarded expression / branch cannot depend on more than one property
                    // - C: Only allow a single (unnamed) property (the "ambient state")
                    //      - forces the user to create an unrelated struct
                    //      - note: the struct can contain references instead of copies?
                    //          - e.g.
                    //  nodegraph <- NodeGraph { nodes: &nodes, connections: &connections }
                    //          - difficult to propagate the changes
                    //
                    // go with option B

                    [desc = Node::description in node, shortcut = Node::shortcut in node] .shortcut = shortcut;
                )

                Button(.label="hello")         // (shortcut for .content.set()), default guard is rev.focus(Unit)
                Button(.label="world")
            }
        }
    }


    /* kyv::VBox::new(vec![
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
    ));*/

    //db.add_watcher(root.clone());

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
