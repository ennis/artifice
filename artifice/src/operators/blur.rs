use crate::{
    eval::{
        imaging::{ImageInputRequest, OpImaging, OpImagingCtx, PxSizeI, RegionOfDefinition, RequestWindow, TiSize},
        EvalError, Image2DCreateInfo, OpGpuDeviceCtx,
    },
    model::Node,
};
use async_trait::async_trait;
use kyute::{
    graal,
    graal::{vk, RecordingContext},
};
use kyute_common::Size;

struct OpBlur {
    /*/// Standard deviation of the gaussian kernel. Also determines the size in pixels of the kernel.
    //#[schema(name = "standardDeviation", default_value = 2.0, ui(min = 0.1))]
    standard_deviation: Attribute<f64>,
    /// Input image.
    // `Image` is a dummy type here
    //#[schema(name = "input", ui(input_bus = "main"))]
    input: Attribute<Image>,*/
}

struct BlurPipeline {
    // Pipeline
    // DescriptorSetLayout
    // DescriptorSet
    //
    pipeline: vk::Pipeline,
}

fn half_gaussian_window_size(std_dev: f64) -> u32 {
    (3.0 * std_dev).ceil() as u32
}

#[async_trait]
impl OpImaging for OpBlur {
    /// Computes the input image regions necessary to evaluate the given region of interest.
    async fn compute_input_requests(
        &self,
        ctx: &OpImagingCtx,
        request: &RequestWindow,
    ) -> Result<Vec<ImageInputRequest>, EvalError> {
        let input = ctx.mandatory_connected_input("input")?;

        let standard_deviation: f64 = ctx.eval_attribute("standardDeviation", ctx.time).await?;
        let kernel_size = half_gaussian_window_size(standard_deviation);

        // expand requested region
        let ks = kernel_size as f64 * 2.0;
        let expanded_roi = request.roi.inflate(ks, ks);
        // FIXME this is wrong (pixel density, pixel aspect ratio)
        // TODO move to method of RequestWindow
        let expanded_resolution = PxSizeI::new(
            request.resolution.width + 2 * kernel_size as i32,
            request.resolution.height + 2 * kernel_size as i32,
        );

        Ok(vec![ImageInputRequest {
            path: input,
            time: ctx.time,
            window: RequestWindow {
                roi: expanded_roi,
                resolution: expanded_resolution,
            },
        }])
    }

    /// Computes the region of definition of the operator.
    async fn compute_region_of_definition(&self, ctx: &OpImagingCtx) -> Result<RegionOfDefinition, EvalError> {
        let standard_deviation: f64 = ctx.eval_attribute("standardDeviation", ctx.time).await?;
        let kernel_size = half_gaussian_window_size(standard_deviation);
        let input_rod = ctx.compute_input_region_of_definition("input").await?;

        // expand requested region
        let ks = kernel_size as f64 * 2.0;
        let expanded_rod = input_rod.rect.inflate(ks, ks);
        // FIXME this is wrong (pixel density, pixel aspect ratio)
        // TODO move to method of RequestWindow
        let expanded_resolution = input_rod
            .native_resolution
            .map(|res| PxSizeI::new(res.width + 2 * kernel_size as i32, res.height + 2 * kernel_size as i32));

        Ok(RegionOfDefinition {
            rect: expanded_rod,
            native_resolution: expanded_resolution,
        })
    }

    /// Computes the blurred image using a GPU device context.
    ///
    /// # Arguments
    /// * rod the region of definition of this operator
    /// * request the area of the image to compute
    /// * input_requests the result of compute_input_requests for the specified request
    async fn device_compute(&self, ctx: &OpImagingCtx, request: &RequestWindow) -> Result<OpImagingResult, EvalError> {
        // Compute the input image.
        //
        // The size of the result is determined by compute_input_requests.
        // ... or should the operator call compute_input_requests by itself?
        //
        // Since it's a "device" function, it's not actually starting any work: instead, it's adding
        // passes on the GPU device frame.
        // (it might start CPU work though)

        // OpImagingCtx:
        // * device_compute_input_image(name, request)
        //
        // OpCtx:
        // * device_add_pass()
        // * device_create_image()
        // * device_create_buffer()
        // * (internal) device_create_output_image()
        // * (internal) device_create_output_buffer()

        let input_image = ctx.device_compute_input_image("input", request).await?;

        // allocate the output image
        let output_image = ctx.device_create_image_2d(
            graal::MemoryLocation::GpuOnly,
            &graal::ImageResourceCreateInfo {
                image_type: vk::ImageType::TYPE_2D,
                usage: vk::ImageUsageFlags::STORAGE,
                format: vk::Format::R32_SFLOAT,
                extent: Default::default(),
                mip_levels: 0,
                array_layers: 0,
                samples: 0,
                tiling: vk::ImageTiling::OPTIMAL,
            },
        );

        let pass = graal::PassBuilder::new()
            //.queue(graal::PassType::Compute)
            .name("blur H pass")
            .image_dependency(
                input_image,
                vk::AccessFlags::SHADER_READ,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            )
            .image_dependency(
                output_image,
                vk::AccessFlags::SHADER_WRITE,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::ImageLayout::GENERAL,
                vk::ImageLayout::GENERAL,
            )
            .record_callback(Box::new(
                move |ctx: &mut RecordingContext, _, cmd_buf: vk::CommandBuffer| {
                    // TODO: add passes
                },
            ));
        ctx.device_add_pass(pass);

        Ok(OpImagingResult::new().add_output_device_image("output", output_image))
    }
}
