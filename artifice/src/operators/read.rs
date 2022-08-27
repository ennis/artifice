use crate::eval::{
    imaging::{
        DeviceComputeImageResult, ImageInputRequest, ImagingOperatorRegistration, OpImaging, OpImagingCtx, PxSizeI,
        RegionOfDefinition, RequestWindow, TiPoint, TiRect, TiSize,
    },
    EvalError, TaskMap,
};
use async_trait::async_trait;
use futures::{future, TryFutureExt};
use kyute::{
    graal,
    graal::{vk, ImageId},
};
use kyute_common::Atom;
use once_cell::sync::Lazy;
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

// problem: these stay loaded, even outside evaluations
// (which is ok, actually)
static IMAGE_READ_TASKS: Lazy<TaskMap<PathBuf, ImageDataLoadResult>> = Lazy::new(|| TaskMap::new());
static IMAGE_READ_HEADER_TASKS: Lazy<TaskMap<PathBuf, ImageHeaderLoadResult>> = Lazy::new(|| TaskMap::new());

pub struct OpRead;

impl OpRead {
    /// Returns the headers for the image at the given file path.
    async fn get_image_headers(&self, path: impl Into<PathBuf>) -> Result<ImageHeaders, EvalError> {
        let path = path.into();

        IMAGE_READ_HEADER_TASKS
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

/// Converts an OIIO type descriptor into its preferred vulkan image format.
///
/// Returns a tuple consisting of the vulkan image format and the pixel stride (size of one pixel in bytes).
/// Panics if the format is not supported.
///
/// Note that the returned vulkan format may not exactly match the layout of pixels in the input file:
/// notably, 3-channel image formats have limited capabilities in vulkan,
/// so this returns the 4-channel format instead when the input image is RGB.
fn oiio_typedesc_to_vk_format(typedesc: &openimageio::TypeDesc, num_channels: usize) -> (vk::Format, usize) {
    let (vk_format, bpp) = match (typedesc, num_channels) {
        (&openimageio::TypeDesc::U8, 1) => (vk::Format::R8_UNORM, 1usize),
        (&openimageio::TypeDesc::U8, 2) => (vk::Format::R8G8_UNORM, 2usize),
        (&openimageio::TypeDesc::U8, 3) => (vk::Format::R8G8B8A8_UNORM, 4usize), // RGB8 not very well supported
        (&openimageio::TypeDesc::U8, 4) => (vk::Format::R8G8B8A8_UNORM, 4usize),
        (&openimageio::TypeDesc::U16, 1) => (vk::Format::R16_UNORM, 2usize),
        (&openimageio::TypeDesc::U16, 2) => (vk::Format::R16G16_UNORM, 4usize),
        (&openimageio::TypeDesc::U16, 3) => (vk::Format::R16G16B16A16_UNORM, 8usize),
        (&openimageio::TypeDesc::U16, 4) => (vk::Format::R16G16B16A16_UNORM, 8usize),
        (&openimageio::TypeDesc::U32, 1) => (vk::Format::R32_UINT, 4usize),
        (&openimageio::TypeDesc::U32, 2) => (vk::Format::R32G32_UINT, 8usize),
        (&openimageio::TypeDesc::U32, 3) => (vk::Format::R32G32B32A32_UINT, 16usize),
        (&openimageio::TypeDesc::U32, 4) => (vk::Format::R32G32B32A32_UINT, 16usize),
        (&openimageio::TypeDesc::HALF, 1) => (vk::Format::R16_SFLOAT, 2usize),
        (&openimageio::TypeDesc::HALF, 2) => (vk::Format::R16G16_SFLOAT, 4usize),
        (&openimageio::TypeDesc::HALF, 3) => (vk::Format::R16G16B16A16_SFLOAT, 8usize),
        (&openimageio::TypeDesc::HALF, 4) => (vk::Format::R16G16B16A16_SFLOAT, 8usize),
        (&openimageio::TypeDesc::FLOAT, 1) => (vk::Format::R32_SFLOAT, 4usize),
        (&openimageio::TypeDesc::FLOAT, 2) => (vk::Format::R32G32_SFLOAT, 8usize),
        (&openimageio::TypeDesc::FLOAT, 3) => (vk::Format::R32G32B32A32_SFLOAT, 16usize),
        (&openimageio::TypeDesc::FLOAT, 4) => (vk::Format::R32G32B32A32_SFLOAT, 16usize),
        _ => panic!("unsupported image format"),
    };
    (vk_format, bpp)
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
        let file_path: String = ctx.eval_attribute("input:file", ctx.time).await?;
        let header = self.get_image_headers(file_path).await?;

        // for images without physical dimensions, assume that their physical size happens to be
        // equal to their pixel size (equivalently, assume a DPI of 96).
        Ok(RegionOfDefinition {
            rect: TiRect::new(
                TiPoint::origin(),
                TiSize::new(header.width as f64, header.height as f64),
            ),
            native_resolution: Some(PxSizeI::new(header.width as i32, header.height as i32)),
        })
    }

    async fn device_compute_image(
        &self,
        ctx: &OpImagingCtx,
        request: &RequestWindow,
    ) -> Result<DeviceComputeImageResult, EvalError> {
        // TODO: honor the request window

        // open the image file
        let file_path: String = ctx.eval_attribute("input:file", ctx.time).await?;

        // TODO: open may fail due to an I/O error, but OpenImageIO doesn't return I/O errors, unfortunately
        let image_input = openimageio::ImageInput::open(&file_path).map_err(|e| EvalError::general(e.to_string()))?;
        let spec = image_input.spec();
        let num_channels = spec.num_channels();
        let format_typedesc = spec.format();
        let width = spec.width();
        let height = spec.height();

        if num_channels > 4 {
            return Err(EvalError::General(format!(
                "unsupported number of channels: {}",
                num_channels
            )));
        }

        let mip_levels = graal::get_mip_level_count(width, height);
        let (vk_format, bytes_per_pixel) = oiio_typedesc_to_vk_format(&format_typedesc, num_channels);

        trace!("reading image file: {file_path}, {width}x{height}, {vk_format:?}, {bytes_per_pixel} bpp");

        // create the texture
        let output_image = ctx.device_create_image(
            graal::MemoryLocation::GpuOnly,
            &graal::ImageResourceCreateInfo {
                image_type: vk::ImageType::TYPE_2D,
                // TODO: get default usage flags somewhere
                usage: vk::ImageUsageFlags::TRANSFER_SRC
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::STORAGE
                    | vk::ImageUsageFlags::SAMPLED,
                format: vk_format,
                extent: vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                },
                // TODO: may not want all mip levels
                mip_levels: 1,
                array_layers: 1,
                samples: 1,
                tiling: Default::default(),
            },
        )?;

        let byte_size = width as u64 * height as u64 * bytes_per_pixel as u64;

        // allocate a staging buffer big enough to receive the pixel data
        let staging_buffer = ctx.device_create_buffer(
            graal::MemoryLocation::CpuToGpu,
            &graal::BufferResourceCreateInfo {
                usage: vk::BufferUsageFlags::TRANSFER_SRC,
                byte_size,
                map_on_create: true,
            },
        )?;

        // read image data directly into the staging buffer
        // FIXME: this is blocking!
        unsafe {
            image_input
                .read_unchecked(
                    0,
                    0,
                    0..num_channels,
                    format_typedesc,
                    staging_buffer.mapped_ptr.unwrap().as_ptr() as *mut u8,
                    bytes_per_pixel,
                )
                .expect("failed to read image");
        }

        let staging_buffer_handle = staging_buffer.handle;

        // upload
        let upload_pass = graal::PassBuilder::new()
            .name("image upload")
            .image_dependency(
                output_image.id,
                vk::AccessFlags::TRANSFER_WRITE,
                vk::PipelineStageFlags::TRANSFER,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            )
            .buffer_dependency(
                staging_buffer.id,
                vk::AccessFlags::TRANSFER_READ,
                vk::PipelineStageFlags::TRANSFER,
            )
            .record_callback(Box::new(move |context, _, command_buffer| unsafe {
                let device = context.vulkan_device();
                let regions = &[vk::BufferImageCopy {
                    buffer_offset: 0,
                    buffer_row_length: width,
                    buffer_image_height: height,
                    image_subresource: vk::ImageSubresourceLayers {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        mip_level: 0,
                        base_array_layer: 0,
                        layer_count: 1,
                    },
                    image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
                    image_extent: vk::Extent3D {
                        width,
                        height,
                        depth: 1,
                    },
                }];

                device.cmd_copy_buffer_to_image(
                    command_buffer,
                    staging_buffer_handle,
                    output_image.handle,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    regions,
                );
            }));
        ctx.device_add_pass(upload_pass)?;

        Ok(
            DeviceComputeImageResult::new(TiRect::new(TiPoint::zero(), TiSize::new(width as f64, height as f64)))
                .plane(
                    "out",
                    PxSizeI::new(width as i32, height as i32),
                    vk_format,
                    output_image.id,
                    output_image.handle,
                ),
        )
    }
}

inventory::submit! {
    ImagingOperatorRegistration {
        name: "read",
        op: &OpRead
    }
}
