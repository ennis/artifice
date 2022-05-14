use crate::eval::{
    imaging::{
        ImageInputRequest, OpImaging, OpImagingCtx, PxSize, RegionOfDefinition, RequestWindow, TiPoint, TiRect, TiSize,
    },
    EvalError, TaskMap,
};
use async_trait::async_trait;
use futures::{future, TryFutureExt};
use kyute_common::Atom;
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::task;

/// Loaded image data
#[derive(Copy, Clone, Debug)]
pub struct ImageHeaders {
    pub width: u32,
    pub height: u32,
}

/// Loaded image data
#[derive(Clone, Debug)]
pub struct ImageData {
    headers: ImageHeaders,
    data: Arc<[u8]>,
}

/// Result type of an image load task
type ImageDataLoadResult = Result<ImageData, EvalError>;
type ImageHeaderLoadResult = Result<ImageHeaders, EvalError>;

// FIXME: we could move those into the general ctx instead of each op maintaining their own list
// -> task with typed ID
// - T -> TaskMap<T, val>
struct OpRead {
    // problem: these stay loaded, even outside evaluations
    // (which is ok, actually)
    /// Image data futures.
    image_data: TaskMap<PathBuf, ImageDataLoadResult>,
    /// Image header futures.
    image_headers: TaskMap<PathBuf, ImageHeaderLoadResult>,
}

impl OpRead {
    /// Returns the headers for the image at the given file path.
    async fn get_image_headers(&self, path: impl Into<PathBuf>) -> Result<ImageHeaders, EvalError> {
        let path = path.into();

        self.image_headers
            .fetch_or_spawn_blocking(path.clone(), || {
                let image_input = openimageio::ImageInput::open(path).map_err(|e| EvalError::general(e.to_string()))?;
                let spec = image_input.spec();
                Ok(ImageHeaders {
                    width: spec.width(),
                    height: spec.height(),
                })
            })
            .await
            .unwrap()
    }
}

#[async_trait]
impl OpImaging for OpRead {
    async fn compute_input_requests(
        &self,
        ctx: &OpImagingCtx,
        request: &RequestWindow,
    ) -> Result<Vec<ImageInputRequest>, EvalError> {
        Ok(vec![])
    }

    async fn compute_region_of_definition(&self, ctx: &OpImagingCtx) -> Result<RegionOfDefinition, EvalError> {
        let file_path: String = ctx.eval_attribute("filePath", ctx.time).await?;
        let header = self.get_image_headers(file_path).await?;

        // for images without physical dimensions, assume that their physical size happens to be
        // equal to their pixel size (equivalently, assume a DPI of 96).
        Ok(RegionOfDefinition {
            rect: TiRect::new(
                TiPoint::origin(),
                TiSize::new(header.width as f64, header.height as f64),
            ),
            native_resolution: Some(PxSize::new(header.width as i32, header.height as i32)),
        })
    }
}
