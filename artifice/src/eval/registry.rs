use crate::eval::EvalError;
use anyhow::anyhow;
use kyute_common::Atom;
use parking_lot::Mutex;
use std::collections::HashMap;

pub struct Registry<T> {
    operators: Mutex<HashMap<Atom, &'static T>>,
}

impl<T> Registry<T> {
    pub fn new() -> Registry<T> {
        Registry {
            operators: Default::default(),
        }
    }
}

impl<T: ?Sized + Sync> Registry<T> {
    /// Registers an imaging operator by name.
    pub fn register(&self, name: &Atom, op: &'static T) -> Result<(), EvalError> {
        let mut map = self.operators.lock();
        if map.contains_key(name) {
            return Err(EvalError::Other(anyhow!(
                "an operator with the same name has already been registered"
            )));
        }
        map.insert(name.clone(), op);
        Ok(())
    }

    /// Returns a reference to a previously registered imaging operator.
    pub fn get(&self, name: &Atom) -> Option<&'static T> {
        let map = self.operators.lock();
        map.get(name).cloned()
    }
}

macro_rules! operator_registry {
    ($registry_name:ident < $op_trait:ident > ) => {
        lazy_static::lazy_static! {
            pub static ref $name: Registry<dyn $op_trait + Sync> = Registry::new();
        }
    };
}

pub(crate) use operator_registry;
