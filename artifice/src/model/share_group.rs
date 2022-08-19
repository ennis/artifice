use crate::model::{path::Path, value::Value};
use imbl::Vector;
use kyute::Data;

/// A group of attributes sharing a common value.
#[derive(Clone, Debug)]
pub struct ShareGroup {
    shares: Vector<Path>,
    value: Value,
}
