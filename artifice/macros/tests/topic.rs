use artifice::util::MessageBus;
use artifice_macros::topic;
use std::{cell::RefCell, rc::Rc};

#[topic(DocumentEvents)]
pub trait DocumentEventListener {
    fn before_change(&mut self);
    fn after_change(&mut self);
}

#[derive(Debug)]
struct TestEventListener;

impl Drop for TestEventListener {
    fn drop(&mut self) {
        eprintln!("dropped")
    }
}

impl DocumentEventListener for TestEventListener {
    fn before_change(&mut self) {
        eprintln!("before_change");
    }

    fn after_change(&mut self) {
        eprintln!("after_change");
    }
}

#[test]
fn test_simple() {
    let bus = MessageBus::new();
    let listener = Rc::new(RefCell::new(TestEventListener));
    DocumentEvents::listen(&bus, listener.clone());
    let mut publisher = DocumentEvents::publisher(&bus);
    publisher.before_change();
    publisher.after_change();
}
