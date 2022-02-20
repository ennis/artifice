use crate::{cache, composable, Cx, EventCtx, Key};
use std::{
    cell::{Cell, RefCell},
    future::Future,
};

/// FIXME: verify that the automatic clone impl doesn't have sketchy implications w.r.t. cache invalidation
#[derive(Clone, Debug)]
pub struct Signal<T> {
    fetched: Cell<bool>,
    value: RefCell<Option<T>>,
    key: cache::Key<Option<T>>,
}

impl<T: Clone + 'static> Signal<T> {
    #[composable]
    pub fn new(cx: Cx) -> Signal<T> {
        let key = cx.state(|| None);
        Signal {
            fetched: Cell::new(false),
            value: RefCell::new(None),
            key,
        }
    }

    #[composable]
    fn fetch_value(&self, cx: Cx) {
        if !self.fetched.get() {
            let value = self.key.get(cx);
            if value.is_some() {
                self.key.set(cx, None);
            }
            self.value.replace(value);
            self.fetched.set(true);
        }
    }

    pub fn set(&self, value: T) {
        self.value.replace(Some(value));
        self.fetched.set(true);
    }

    pub fn signal(&self, ctx: &mut EventCtx, value: T) {
        ctx.set_state(self.key, Some(value));
    }

    #[composable]
    pub fn signalled(&self, cx: Cx) -> bool {
        self.fetch_value(cx);
        self.value.borrow().is_some()
    }

    #[composable]
    pub fn value(&self, cx: Cx) -> Option<T> {
        self.fetch_value(cx);
        self.value.borrow().clone()
    }
}

pub struct State<T> {
    key: cache::Key<T>,
}

impl<T: Clone + 'static> State<T> {
    #[composable]
    pub fn new(cx: Cx, init: impl FnOnce() -> T) -> State<T> {
        let key = cx.state(init);
        State { key }
    }

    #[composable]
    pub fn get(&self, cx: Cx) -> T {
        self.key.get(cx)
    }

    #[composable]
    pub fn update(&self, cx: Cx, value: Option<T>) {
        if let Some(value) = value {
            self.key.set(cx, value)
        }
    }

    #[composable]
    pub fn set(&self, cx: Cx, value: T) {
        self.key.set(cx, value)
    }
}
