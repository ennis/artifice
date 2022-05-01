//! Imaging evaluation context
use crate::{
    eval::{registry::operator_registry, EvalError, EvalKey, GeneralEvalState, OpCtx, TaskMap},
    model::{metadata, Document, ModelPath, Node},
};
use anyhow::anyhow;
use async_trait::async_trait;
use futures::{future::Shared, FutureExt};
use kyute_common::{Atom, Rect, SizeI, Transform};
use lazy_static::lazy_static;
use parking_lot::{Mutex, RwLock};
use std::{
    cell::RefCell,
    collections::HashMap,
    future::Future,
    hash::{Hash, Hasher},
    ops::Deref,
    pin::Pin,
    sync::{Arc, Weak},
};
use tokio::task;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Imaging operators registration
////////////////////////////////////////////////////////////////////////////////////////////////////

operator_registry!(IMAGING_OPERATORS<OpImaging>);

/*lazy_static! {
    static ref IMAGING_OPERATORS: Mutex<HashMap<Atom, &'static (dyn OpImaging + Sync)>> = Mutex::new(HashMap::new());
}

/// Registers an imaging operator by name.
pub fn register_imaging_operator(name: &Atom, op: &'static (dyn OpImaging + Sync)) -> Result<(), EvalError> {
    let mut map = IMAGING_OPERATORS.lock();
    if map.contains_key(name) {
        return Err(EvalError::Other(anyhow!(
            "an operator with the same name has already been registered"
        )));
    }
    map.insert(name.clone(), op);
    Ok(())
}

/// Returns a reference to a previously registered imaging operator.
pub fn get_imaging_operator(name: &Atom) -> Option<&'static (dyn OpImaging + Sync)> {
    let map = IMAGING_OPERATORS.lock();
    map.get(name).cloned()
}*/

////////////////////////////////////////////////////////////////////////////////////////////////////
// Units
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Target-independent pixel (TIP) unit. Defined to be equal to 1/96 inch.
///
/// This is the unit of measure for regions (RoD,RoI) of imaging operators.
/// It is a physical measure of length, convertible to mm or inches, for example.
/// As the name suggests, it is independent of any *target resolution*.
pub struct Tip;

/// Pixel unit. Represents one pixel on some target surface.
///
/// Values with this unit always integral, and represent a size or offset as a number of pixels
/// on some target surface. The physical size of such a size in pixels depends on the pixel density of the target,
/// and the _pixel aspect ratio_ (i.e. the shape of the pixel).
///
/// Lengths with this unit typically cannot be infinite.
pub struct Px;

pub type TiRect = euclid::Rect<f64, Tip>;
pub type TiPoint = euclid::Point2D<f64, Tip>;
pub type TiSize = euclid::Size2D<f64, Tip>;
pub type TiOffset = euclid::Vector2D<f64, Tip>;

pub type PxRect = euclid::Rect<i32, Px>;
pub type PxPoint = euclid::Point2D<i32, Px>;
pub type PxSize = euclid::Size2D<i32, Px>;
pub type PxOffset = euclid::Vector2D<i32, Px>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// ImageRequest
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Describes a region of interest on an image, and a target resolution for the rendered image.
#[derive(Copy, Clone, Debug)]
pub struct RequestWindow {
    /// Requested region of interest in local, target-independent coordinates.
    pub roi: TiRect,
    /// Requested resolution (in device pixels). Combined with the RoI above, this also defines a pixel aspect ratio and pixel density.
    pub resolution: PxSize,
}

impl RequestWindow {
    /// Returns the aspect ratio of the window.
    pub fn aspect_ratio(&self) -> f64 {
        self.roi.width() / self.roi.height()
    }

    /// Returns the pixel aspect ratio.
    pub fn pixel_aspect_ratio(&self) -> f64 {
        (self.roi.width() * self.resolution.height as f64) / (self.roi.height() * self.resolution.width as f64)
    }

    /// Returns the size of a pixel in local target-independent units.
    pub fn pixel_size(&self) -> TiSize {
        TiSize::new(
            self.roi.width() / self.resolution.width as f64,
            self.roi.height() / self.resolution.height as f64,
        )
    }
}

impl PartialEq for RequestWindow {
    fn eq(&self, other: &Self) -> bool {
        self.roi.origin.x.to_bits() == other.roi.origin.x.to_bits()
            && self.roi.origin.y.to_bits() == other.roi.origin.y.to_bits()
            && self.roi.size.width.to_bits() == other.roi.size.width.to_bits()
            && self.roi.size.height.to_bits() == other.roi.size.height.to_bits()
            && self.resolution == other.resolution
    }
}

impl Eq for RequestWindow {}

impl Hash for RequestWindow {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.roi.origin.x.to_bits());
        state.write_u64(self.roi.origin.y.to_bits());
        state.write_u64(self.roi.size.width.to_bits());
        state.write_u64(self.roi.size.height.to_bits());
        self.resolution.hash(state);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// OpImaging + Ctx
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A request for some region of an input image.
pub struct ImageInputRequest {
    pub path: ModelPath,
    pub time: f64,
    pub window: RequestWindow,
}

/// Region of definition returned by `OpImaging::compute_region_of_definition`.
pub struct RegionOfDefinition {
    /// The target-independent region of definition.
    pub rect: TiRect,
    /// The "native" target resolution of the operator.
    ///
    /// For example, an operator that loads an image file may want to set this field to the
    /// pixel size of the image file. This can be important in "pixel-perfect" rendering pipelines
    /// to avoid unwanted resampling.
    pub native_resolution: Option<PxSize>,
}

/// Arguments for `OpImaging::compute_input_requests`.
pub struct InputRequestsArgs {
    /// Evaluation time.
    pub time: f64,
    /// Region of interest requested by the calling operator.
    pub roi: RequestWindow,
}

/// Imaging operators.
#[async_trait]
pub trait OpImaging {
    /// Computes the input image regions necessary to evaluate the given region of interest.
    async fn compute_input_requests(
        &self,
        ctx: &OpImagingCtx,
        request: &RequestWindow,
    ) -> Result<Vec<ImageInputRequest>, EvalError>;

    /// Computes the region of definition of the operator.
    async fn compute_region_of_definition(&self, ctx: &OpImagingCtx) -> Result<RegionOfDefinition, EvalError>;
}

/// Arguments for evaluating an image.
#[derive(Copy, Clone, Debug)]
pub struct ImagingEvalArgs {
    roi: Rect,
}

/// Context passed to `OpImaging` operators.
pub struct OpImagingCtx {
    op_ctx: OpCtx,
    /// Current image transform (local coords to target).
    pub transform: Transform,
}

impl Deref for OpImagingCtx {
    type Target = OpCtx;
    fn deref(&self) -> &Self::Target {
        &self.op_ctx
    }
}

impl OpImagingCtx {
    /// Returns the evaluation time.
    pub fn time(&self) -> f64 {
        self.general.time()
    }

    /// Computes the region of definition of the image at the specified model path.
    pub async fn compute_input_region_of_definition(
        &self,
        input: impl Into<Atom>,
    ) -> Result<RegionOfDefinition, EvalError> {
        self.compute_input_region_of_definition_at_time(input, self.time).await
    }

    /// Computes the region of definition of the image at the specified model path.
    pub async fn compute_input_region_of_definition_at_time(
        &self,
        input: impl Into<Atom>,
        time: f64,
    ) -> Result<RegionOfDefinition, EvalError> {
        let path = self.node.base.path.join_attribute(input);
        self.op_ctx
            .compute_region_of_definition(self.transform, path, time)
            .await
    }

    pub fn request_input(&mut self, path: &ModelPath, time: f64, roi: Rect) {
        // Get or create a request for the image
        let imaging_ctx = self.eval.imaging.as_mut().unwrap();
        let req = imaging_ctx.get_or_create_request(path, time);
        // See if there's already a region
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Image requests
////////////////////////////////////////////////////////////////////////////////////////////////////

slotmap::new_key_type! {
    /// ID of an image request.
    pub struct ImageRequestId;
}

/*/// Requested regions of an image during evaluation.
pub struct ImageRequest {
    /// Path to the model object (should implement OpImaging) the produces the image.
    path: ModelPath,
    /// All requested regions of the image.
    regions: Vec<Rect>,
}*/

/// Data uniquely identifying a request for some data from an image.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ImageRequestKey {
    /// Path to the model object (should implement OpImaging) the produces the image.
    pub model_path: ModelPath,
    /// Evaluation time.
    pub time: f64,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ImagingEvalCtx
////////////////////////////////////////////////////////////////////////////////////////////////////

type EvalFuture<T> = Shared<Pin<Box<dyn Future<Output = Result<T, EvalError>>>>>;

/// Type of an evaluation future for `compute_region_of_definition`.
type RodFuture = EvalFuture<RegionOfDefinition>;

/// Imaging context. Owned internally by `EvalSession`.
pub(crate) struct ImagingEvalState {
    /// Tasks spawned by `compute_region_of_definition`.
    rod_tasks: TaskMap<EvalKey, Result<RegionOfDefinition, EvalError>>,
}

impl ImagingEvalState {
    /// Creates a new instance.
    pub(crate) fn new() -> ImagingEvalState {
        ImagingEvalState {
            rod_tasks: TaskMap::new(),
        }
    }

    /*/// Creates or returns the existing `ImageRequest` for the given model path at the given time.
    pub(crate) fn get_or_create_request(&mut self, model_path: &ModelPath, time: f64) -> &mut ImageRequest {
        // try to find an existing request
        let key = ImageRequestKey {
            model_path: model_path.clone(),
            time,
        };
        let id = *self
            .ids
            .entry(key)
            .or_insert_with(|| self.requests.insert(ImageRequest::new(model_path.clone())));
        let req = self.requests.get_mut(id).unwrap();
        req
    }*/
}

impl OpCtx {
    /// Computes the region of definition of the image at the specified model path.
    pub async fn compute_region_of_definition(
        &self,
        transform: Transform,
        path: ModelPath,
        time: f64,
    ) -> Result<RegionOfDefinition, EvalError> {
        let key = EvalKey { path, time };

        let this = self.clone();
        let document = self.document();

        self.rod_tasks
            .fetch_or_spawn(key, async move {
                // get the imaging operator for the target path
                let node = document.find_node(&path).ok_or(EvalError::PathNotFound(path.clone()))?;
                let op_name = node
                    .get_metadata(metadata::OPERATOR)
                    .ok_or_else(|| EvalError::Other(anyhow!("target has no operator")))?;
                let op = IMAGING_OPERATORS
                    .get(&op_name)
                    .ok_or(EvalError::UnknownOperator(op_name))?;

                // issue: can't borrow self from another task
                let mut op_ctx = OpImagingCtx {
                    imaging_ctx: this,
                    document: &document,
                    node,
                    transform,
                    roi: Default::default(),
                };

                // invoke operator here
                op.compute_region_of_definition(&op_ctx).await
            })
            .await
    }
}
