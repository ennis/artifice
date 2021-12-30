use crate::model::{path::ModelPath, value::Value};
use imbl::Vector;

/// A group of properties sharing a common value.
#[derive(Clone, Data)]
pub struct ShareGroup {
    shares: Vector<ModelPath>,
    value: Value,
}
