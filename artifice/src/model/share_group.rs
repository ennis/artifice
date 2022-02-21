use crate::model::{path::ModelPath, value::Value};
use imbl::Vector;
use kyute::Data;

/// A group of attributes sharing a common value.
#[derive(Clone, Debug, Data)]
pub struct ShareGroup {
    shares: Vector<ModelPath>,
    value: Value,
}
