use crate::{
    eval::{
        imaging::{ImageInputRequest, OpImaging, OpImagingCtx, PxSize, RegionOfDefinition, RequestWindow, TiSize},
        EvalError,
    },
    model::{Attribute, Image, Node},
};
use kyute_common::Size;

struct OpBlur {
    /// Standard deviation of the gaussian kernel. Also determines the size in pixels of the kernel.
    //#[schema(name = "standardDeviation", default_value = 2.0, ui(min = 0.1))]
    standard_deviation: Attribute<f64>,
    /// Input image.
    // `Image` is a dummy type here
    //#[schema(name = "input", ui(input_bus = "main"))]
    input: Attribute<Image>,
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
        let standard_deviation: f64 = ctx.eval_attribute("standardDeviation", ctx.time).await?;
        let input = ctx.get_source("input");
        let kernel_size = half_gaussian_window_size(standard_deviation);

        // expand requested region
        let ks = kernel_size as f64 * 2.0;
        let expanded_roi = request.roi.inflate(ks, ks);
        // FIXME this is wrong (pixel density, pixel aspect ratio)
        // TODO move to method of RequestWindow
        let expanded_resolution = PxSize::new(
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
            .map(|res| PxSize::new(res.width + 2 * kernel_size as i32, res.height + 2 * kernel_size as i32));

        Ok(RegionOfDefinition {
            rect: expanded_rod,
            native_resolution: expanded_resolution,
        })
    }
}
