use kyute::{
    composable, graal,
    graal::{
        vk::{Extent3D, Format, ImageTiling, ImageType, ImageUsageFlags},
        MemoryLocation,
    },
    shell::platform::Platform,
    BoxConstraints, Environment, Event, EventCtx, GpuCtx, LayoutCtx, Measurements, PaintCtx, Rect,
    Widget, WidgetPod,
};
use std::cell::RefCell;

pub struct View3D {
    target_format: Format,
    target_image: RefCell<Option<graal::ImageInfo>>,
}

impl View3D {
    #[composable]
    pub fn new(format: Format) -> WidgetPod<View3D> {
        WidgetPod::new(View3D {
            target_format: format,
            target_image: RefCell::new(None),
        })
    }
}

impl Drop for View3D {
    fn drop(&mut self) {
        // must dealloc target image
        // TODO RAII GPU resources (use `Platform::instance().gpu_context`).
        if let Some(image) = self.target_image.take() {
            let ctx = Platform::instance().gpu_context();
            let mut ctx = ctx.lock().unwrap();
            ctx.destroy_image(image.id);
        }
    }
}

impl Widget for View3D {
    fn event(&self, ctx: &mut EventCtx, event: &mut Event) {}

    fn layout(
        &self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        env: &Environment,
    ) -> Measurements {
        Measurements::new(constraints.max)
    }

    fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, env: &Environment) {
        todo!()
    }

    fn gpu_frame(&self, ctx: &mut GpuCtx) {
        let frame = ctx.frame();
        let target_image = self.target_image.borrow();
        let size = ctx.measurements().size;

        let target_image = if let Some(&image) = &*target_image {
            image
        } else {
            // allocate image
            frame.context().create_image(
                "view3d",
                MemoryLocation::GpuOnly,
                &graal::ImageResourceCreateInfo {
                    image_type: ImageType::TYPE_2D,
                    usage: ImageUsageFlags::COLOR_ATTACHMENT
                        | ImageUsageFlags::TRANSFER_SRC
                        | ImageUsageFlags::SAMPLED,
                    format: Format::R16,
                    extent: Extent3D {
                        width: size.width,
                        height: size.height,
                        depth: 1
                    },
                    mip_levels: 1,
                    array_layers: 1,
                    samples: 1,
                    tiling: ImageTiling::OPTIMAL,
                },
            )
        };
    }
}
