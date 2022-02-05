use artifice::model::{Atom, Node, Nodes};

#[test]
fn test_props() {
    //let mut nodes = Nodes::with_key();

    let mut node = Node::new();

    assert_eq!(node.add_property("a".into(), "i32".into()), Atom::from("a"));
    assert_eq!(
        node.add_property("a".into(), "i32".into()),
        Atom::from("a_0")
    );
    assert_eq!(
        node.add_property("a".into(), "i32".into()),
        Atom::from("a_1")
    );

    assert_eq!(
        node.add_property("b_0".into(), "i32".into()),
        Atom::from("b_0")
    );
    assert_eq!(node.add_property("b".into(), "i32".into()), Atom::from("b"));
    assert_eq!(
        node.add_property("b".into(), "i32".into()),
        Atom::from("b_1")
    );
}
