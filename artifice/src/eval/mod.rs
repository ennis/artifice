mod device;
mod error;
pub mod imaging;
mod pipeline;
mod shader;
mod task_map;
mod variability;

pub use error::EvalError;
pub use task_map::{TaskError, TaskMap};
pub use variability::Variability;

use crate::{
    eval::{
        device::DeviceEvalState,
        error::EvalErrorContextExt,
        imaging::{
            get_imaging_operator, DeviceComputeImageResult, ImagingEvalState, ImagingOperatorRegistration, OpImaging,
            OpImagingCtx, PxSizeI, RequestWindow,
        },
    },
    model::{metadata, Document, Node, Param, Path, Value},
};
use async_trait::async_trait;
use kyute::{
    graal,
    graal::{vk, Device},
    shell::application::Application,
};
use kyute_common::{Atom, SizeI, Transform};
use parking_lot::Mutex;
use std::{
    convert::TryFrom,
    future::Future,
    hash::{Hash, Hasher},
    mem,
    sync::Arc,
    time::Duration,
};
use tokio::task::JoinHandle;

////////////////////////////////////////////////////////////////////////////////////////////////////
// OpGeneral
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Operator trait.
#[async_trait]
pub trait OpGeneral {
    /// Evaluates the specified attribute at the specified time.
    async fn eval(&self, attribute: &Param, time: f64) -> Result<Value, EvalError>;
}

pub struct GeneralOperatorRegistration {
    pub name: &'static str,
    pub op: &'static (dyn OpGeneral + Sync),
}

inventory::collect!(GeneralOperatorRegistration);

pub fn find_general_operator(name: &str) -> Result<&'static (dyn OpGeneral + Sync), EvalError> {
    for op in inventory::iter::<GeneralOperatorRegistration> {
        if op.name == name {
            return Ok(op.op);
        }
    }
    Err(EvalError::UnknownOperator)
}

/// Returns the associated general operator on the given node.
pub fn get_general_operator(node: &Node) -> Result<&'static (dyn OpGeneral + Sync), EvalError> {
    let op_name = node.operator().ok_or(EvalError::NoOperator)?;
    find_general_operator(op_name.as_ref())
}

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
pub struct EvalState {
    document: Document,
    general: GeneralEvalState,
    imaging: ImagingEvalState,
    device_state: DeviceEvalState,
}

impl EvalState {
    async fn device_evaluate_image(
        this: Arc<EvalState>,
        path: &Path,
        time: f64,
        request: &RequestWindow,
    ) -> Result<DeviceComputeImageResult, EvalError> {
        let node = this.document.node(path).ok_or(EvalError::PathNotFound)?;
        let op = get_imaging_operator(&node)?;

        let ctx = OpImagingCtx {
            op_ctx: OpCtx {
                eval: this.clone(),
                node: node.clone(),
                time,
            },
            transform: Transform::identity(),
        };

        op.device_compute_image(&ctx, request).await
    }
}

pub struct Evaluation(Arc<EvalState>);

impl Evaluation {
    pub fn new(device: Arc<graal::Device>, document: Document) -> Evaluation {
        let state = Arc::new(EvalState {
            document,
            general: GeneralEvalState::new(),
            imaging: ImagingEvalState::new(),
            device_state: DeviceEvalState::new(device),
        });
        Evaluation(state)
    }

    /// Evaluates an imaging operator at the specified path.
    pub fn device_evaluate_image(
        &self,
        path: &Path,
        time: f64,
        request: &RequestWindow,
    ) -> Result<DeviceComputeImageResult, EvalError> {
        let runtime_handle = tokio::runtime::Handle::current();
        let result = runtime_handle.block_on(EvalState::device_evaluate_image(self.0.clone(), path, time, request))?;
        // before flushing, extract the final outputs from the transient resource list
        for (_, plane) in result.planes.iter() {
            self.0.device_state.make_image_persistent(plane.id);
        }
        self.0.device_state.flush();
        Ok(result)
    }
}

/// Context passed to operators.
pub struct OpCtx {
    eval: Arc<EvalState>,
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

/// General operator context.
impl OpCtx {
    /// Creates a new opctx
    pub(crate) fn new(eval: Arc<EvalState>, time: f64, node: Node) -> OpCtx {
        OpCtx { eval, time, node }
    }

    /// Returns the path to the attribute connected to the current node's specified input.
    ///
    /// Returns None if the specified input is unconnected.
    pub fn connected_input(&self, input_name: impl Into<Atom>) -> Result<Option<Path>, EvalError> {
        let input_name = input_name.into();
        let attribute = self.node.attribute(&input_name).ok_or(EvalError::PathNotFound)?;
        Ok(attribute.connection.clone())
    }

    /// Same as `connected_input` but returns an error if the specified input is unconnected.
    pub fn mandatory_connected_input(&self, input_name: impl Into<Atom>) -> Result<Path, EvalError> {
        let input_name = input_name.into();
        let path = self
            .connected_input(input_name.clone())?
            .ok_or(EvalError::MandatoryInputUnconnected { input_name })?;
        Ok(path)
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
    async fn eval_any(&self, path: Path, time: f64) -> Result<Value, EvalError> {
        assert!(path.is_attribute());
        trace!("evaluating: {:?}", path);

        let key = EvalKey {
            path: path.clone(),
            time,
        };

        let document = self.eval.document.clone();

        let attribute = document.attribute(&path).ok_or(EvalError::PathNotFound)?;
        if let Some(ref value) = attribute.value {
            Ok(value.clone())
        } else {
            // no value, evaluate attribute using the operator defined on the parent node
            let node = document.node(&path.parent().unwrap()).unwrap();
            let op = get_general_operator(&node)?;
            let attribute = attribute.clone();
            self.eval
                .general
                .tasks
                .fetch_or_spawn(key, async move { op.eval(&attribute, time).await })
                .await
                .unwrap()
        }
    }

    async fn eval_inner<T: TryFrom<Value>>(&self, path: Path, time: f64) -> Result<T, EvalError> {
        let value = self.eval_any(path, time).await?;
        let value = T::try_from(value).map_err(|_| EvalError::ValueConversionError)?;
        Ok(value)
    }

    pub async fn eval<T: TryFrom<Value>>(&self, path: Path, time: f64) -> Result<T, EvalError> {
        self.eval_inner(path.clone(), time)
            .await
            .context(format!("while evaluating {:?}", path))
    }

    //--- DEVICE API -------------------------------------------------------------------------------

    /// Creates a device image.
    ///
    /// The resulting image is only valid for the current evaluation.
    /// The caller does not receive ownership of the returned image.
    pub fn device_create_image(
        &self,
        location: graal::MemoryLocation,
        create_info: &graal::ImageResourceCreateInfo,
    ) -> Result<graal::ImageInfo, EvalError> {
        self.eval.device_state.create_image(location, create_info)
    }

    /// Creates a device buffer.
    ///
    /// The resulting buffer is only valid for the current evaluation.
    /// The caller does not receive ownership of the returned buffer.
    pub fn device_create_buffer(
        &self,
        location: graal::MemoryLocation,
        create_info: &graal::BufferResourceCreateInfo,
    ) -> Result<graal::BufferInfo, EvalError> {
        self.eval.device_state.create_buffer(location, create_info)
    }

    /// Adds a render or compute pass to the current work queue.
    ///
    pub fn device_add_pass(&self, pass: graal::PassBuilder<'static, ()>) -> Result<(), EvalError> {
        self.eval.device_state.add_pass(pass)
    }

    /// Flushes pending operations on the device.
    pub fn device_flush(&self) -> tokio::task::JoinHandle<()> {
        self.eval.device_state.flush()
    }
}
