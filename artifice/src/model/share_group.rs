use crate::model::{path::ModelPath, value::Value};
use imbl::Vector;
use kyute::Data;

/// A group of attributes sharing a common value.
#[derive(Clone, Debug)]
pub struct ShareGroup {
    shares: Vector<ModelPath>,
    value: Value,
}

// TODO imbl data impls
impl Data for ShareGroup {
    fn same(&self, other: &Self) -> bool {
        self.value.same(&other.value) && self.shares.ptr_eq(&other.shares)
    }
}
