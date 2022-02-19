use crate::{cache, composable, EventCtx, Key};
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
    pub fn new() -> Signal<T> {
        let key = #[compose]
        cache::state(|| None);
        Signal {
            fetched: Cell::new(false),
            value: RefCell::new(None),
            key,
        }
    }

    #[composable]
    fn fetch_value(&self) {
        if !self.fetched.get()
        {
            let value = #[compose]
            self.key.get();
            if value.is_some() {
                #[compose]
                self.key.set(None);
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
    pub fn signalled(&self) -> bool {
        #[compose] self.fetch_value();
        self.value.borrow().is_some()
    }

    #[composable]
    pub fn value(&self) -> Option<T> {
        #[compose]
        self.fetch_value();
        self.value.borrow().clone()
    }
}

pub struct State<T> {
    key: cache::Key<T>,
}

impl<T: Clone + 'static> State<T> {
    #[composable]
    pub fn new(init: impl FnOnce() -> T) -> State<T> {
        let key = #[compose]
        cache::state(init);
        State { key }
    }

    #[composable]
    pub fn get(&self) -> T {
        #[compose]
        self.key.get()
    }

    #[composable]
    pub fn update(&self, value: Option<T>) {
        if let Some(value) = value {
            #[compose]
            self.key.set(value)
        }
    }

    #[composable]
    pub fn set(&self, value: T) {
        #[compose]
        self.key.set(value)
    }
}
