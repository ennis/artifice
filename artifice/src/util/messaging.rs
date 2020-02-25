use std::any::Any;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::{Rc, Weak};
use typemap::TypeMap;

pub trait Topic: Any {
    type Listener: ?Sized + 'static;
}

struct TopicWrapper<T: Topic>(PhantomData<T>);
impl<T: Topic> typemap::Key for TopicWrapper<T> {
    type Value = TopicListeners<T::Listener>;
}

struct MessageBusInner {
    topics: TypeMap,
}

#[derive(Clone)]
pub struct MessageBus(Rc<RefCell<MessageBusInner>>);

impl MessageBus {
    pub fn new() -> MessageBus {
        let inner = MessageBusInner {
            topics: TypeMap::new(),
        };
        MessageBus(Rc::new(RefCell::new(inner)))
    }

    /// Registers a topic on this message bus.
    pub fn register<T: Topic>(&self) -> TopicListeners<T::Listener> {
        let mut inner = self.0.borrow_mut();
        inner
            .topics
            .entry::<TopicWrapper<T>>()
            .or_insert_with(|| TopicListeners::new())
            .clone()
    }
}

pub struct TopicListeners<L: ?Sized> {
    inner: Rc<RefCell<Vec<Weak<RefCell<L>>>>>,
}

// yet another #26925 clone impl...
impl<L: ?Sized> Clone for TopicListeners<L> {
    fn clone(&self) -> Self {
        TopicListeners {
            inner: self.inner.clone(),
        }
    }
}

impl<L: ?Sized> TopicListeners<L> {
    pub fn new() -> TopicListeners<L> {
        TopicListeners {
            inner: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn add_listener(&self, l: Rc<RefCell<L>>) {
        if let Ok(mut listeners) = self.inner.try_borrow_mut() {
            listeners.push(Rc::downgrade(&l))
        } else {
            unimplemented!("reentrant call to add_listener")
        }
    }

    pub fn for_each(&self, mut f: impl FnMut(Rc<RefCell<L>>)) {
        let listeners = self.inner.try_borrow().expect("reentrant event submission");
        for l in listeners.iter() {
            f(l.upgrade().expect("listener deleted"))
        }
    }
}
