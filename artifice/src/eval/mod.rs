mod error;
pub mod imaging;
pub mod registry;
mod task_map;

pub use error::EvalError;
pub use registry::OpRegistry;
pub use task_map::TaskMap;

use crate::{
    eval::{imaging::ImagingEvalState, registry::operator_registry},
    model::{metadata, Document, FromValue, ModelPath, Node},
};
use artifice::model::Value;
use async_trait::async_trait;
use futures::{future, FutureExt};
use kyute::graal;
use kyute_common::Atom;
use lazy_static::lazy_static;
use std::{
    any::Any,
    collections::HashMap,
    future::Future,
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};
use tokio::{sync::RwLock, task};

////////////////////////////////////////////////////////////////////////////////////////////////////
// OpGeneral
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Operator trait.
#[async_trait]
pub trait OpGeneral {
    /// Evaluates the specified attribute at the specified time.
    async fn eval(&self, attribute: &AttributeAny, time: f64) -> Result<Value, EvalError>;
}

operator_registry!(GENERAL_OPERATORS<OpGeneral>);

////////////////////////////////////////////////////////////////////////////////////////////////////
// EvalKey
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Key identifying a particular evaluation.
#[derive(Clone, Debug)]
pub struct EvalKey {
    pub path: ModelPath,
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

    /// Evaluates an attribute of the current node.
    pub async fn eval_attribute<T: FromValue>(&self, attribute: impl Into<Atom>, time: f64) -> Result<T, EvalError> {
        self.eval_any(self.node.base.path.join_attribute(attribute), time).await
    }

    /// Evaluates an attribute at the given path, at the given time.
    pub async fn eval_any(&self, path: ModelPath, time: f64) -> Result<Value, EvalError> {
        assert!(path.is_attribute());

        let key = EvalKey { path, time };

        let document = self.eval.document.clone();
        self.tasks
            .fetch_or_spawn(key, async move {
                let attribute = self
                    .document
                    .find_attribute(&path)
                    .ok_or_else(|| EvalError::PathNotFound(path.clone()))?;
                if let Some(ref value) = attribute.value {
                    value.clone()
                } else {
                    // no value, evaluate attribute
                    let parent_node = document.find_node(&path.parent().unwrap()).unwrap();
                    let op_id = parent_node.find_metadata(metadata::OPERATOR).unwrap();
                    let op: &'static dyn OpGeneral = GENERAL_OPERATORS.get(op_id).unwrap();
                    op.eval(attribute, time).await
                }
            })
            .await
    }

    pub async fn eval<T: FromValue>(&self, path: ModelPath, time: f64) -> Result<T, EvalError> {
        let value = self.eval_any(path, time).await?;
        T::from_value(&value).ok_or(EvalError::TypeMismatch)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// EvalCtx
////////////////////////////////////////////////////////////////////////////////////////////////////
