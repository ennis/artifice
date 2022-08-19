mod error;
pub mod imaging;
mod pipeline;
pub mod registry;
mod shader;
mod task_map;
mod variability;

pub use error::EvalError;
pub use registry::Registry;
pub use task_map::{TaskError, TaskMap};
pub use variability::Variability;

use crate::{
    eval::{imaging::ImagingEvalState, registry::operator_registry},
    model::{metadata, Document, Node, Param, Path, Value},
};
use async_trait::async_trait;
use kyute_common::Atom;
use std::{
    convert::TryFrom,
    hash::{Hash, Hasher},
    sync::Arc,
};

////////////////////////////////////////////////////////////////////////////////////////////////////
// OpGeneral
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Operator trait.
#[async_trait]
pub trait OpGeneral {
    /// Evaluates the specified attribute at the specified time.
    async fn eval(&self, attribute: &Param, time: f64) -> Result<Value, EvalError>;
}

operator_registry!(GENERAL_OPERATORS<OpGeneral>);

////////////////////////////////////////////////////////////////////////////////////////////////////
// EvalKey
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Key identifying a particular evaluation.
#[derive(Clone, Debug)]
pub struct EvalKey {
    pub path: Path,
    pub time: f64,
}

impl PartialEq for EvalKey {
    fn eq(&self, other: &Self) -> bool {
        self.path.eq(&other.path) && self.time.to_bits() == other.time.to_bits()
    }
}

impl Eq for EvalKey {}

impl Hash for EvalKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.hash(state);
        self.time.to_bits().hash(state);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// EvalCtx
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Evaluation context.
pub struct EvalCtx {
    document: Document,
    general: GeneralEvalState,
    imaging: ImagingEvalState,
}

impl EvalCtx {
    pub fn new(document: Document) -> EvalCtx {
        EvalCtx {
            document,
            general: GeneralEvalState::new(),
            imaging: ImagingEvalState::new(),
        }
    }
}

/// Context passed to operators.
pub struct OpCtx {
    eval: Arc<EvalCtx>,
    pub time: f64,
    pub node: Node,
}

/// Values produced by a general evaluation operation.
///
/// Specialized evaluation contexts may produce other kinds of values.
pub type GeneralEvalResult = Result<Value, EvalError>;

/// General evaluation state.
struct GeneralEvalState {
    /// Pending or complete evaluation tasks.
    tasks: TaskMap<EvalKey, Result<Value, EvalError>>,
}

impl GeneralEvalState {
    pub fn new() -> GeneralEvalState {
        GeneralEvalState { tasks: TaskMap::new() }
    }
}

/// General operators.
impl OpCtx {
    /// Creates a new opctx
    pub(crate) fn new(eval: Arc<EvalCtx>, time: f64, node: Node) -> OpCtx {
        OpCtx { eval, time, node }
    }

    /// Returns the path to the attribute connected to the current node's specified input.
    ///
    /// Returns None if the specified input is unconnected.
    pub fn connected_input(&self, input_name: impl Into<Atom>) -> Result<Option<Path>, EvalError> {
        let input_name = input_name.into();
        let attribute = self
            .node
            .attribute(&input_name)
            .ok_or_else(|| EvalError::PathNotFound(self.node.path.join_attribute(input_name.clone())))?;
        Ok(attribute.connection.clone())
    }

    /// Same as `connected_input` but returns an error if the specified input is unconnected.
    pub fn mandatory_connected_input(&self, input_name: impl Into<Atom>) -> Result<Path, EvalError> {
        let input_name = input_name.into();
        self.connected_input(input_name.clone())?
            .ok_or(EvalError::MandatoryInputUnconnected { input_name })
    }

    /// Evaluates an attribute of the current node.
    pub async fn eval_attribute<T: TryFrom<Value>>(
        &self,
        attribute: impl Into<Atom>,
        time: f64,
    ) -> Result<T, EvalError> {
        self.eval(self.node.path.join_attribute(attribute), time).await
    }

    /// Evaluates an attribute at the given path, at the given time.
    pub async fn eval_any(&self, path: Path, time: f64) -> Result<Value, EvalError> {
        assert!(path.is_attribute());
        let key = EvalKey {
            path: path.clone(),
            time,
        };

        let document = self.eval.document.clone();
        self.eval
            .general
            .tasks
            .fetch_or_spawn(key, async move {
                let attribute = document
                    .attribute(&path)
                    .ok_or_else(|| EvalError::PathNotFound(path.clone()))?;
                if let Some(ref value) = attribute.value {
                    Ok(value.clone())
                } else {
                    // no value, evaluate attribute
                    let parent_node = document.node(&path.parent().unwrap()).unwrap();
                    let op_id = parent_node.metadata(metadata::OPERATOR).unwrap();
                    let op = GENERAL_OPERATORS.get(&op_id).unwrap();
                    op.eval(attribute, time).await
                }
            })
            .await
            .unwrap()
    }

    pub async fn eval<T: TryFrom<Value>>(&self, path: Path, time: f64) -> Result<T, EvalError> {
        let value = self.eval_any(path, time).await?;
        T::try_from(value).map_err(|_| EvalError::ValueConversionError)
    }
}
