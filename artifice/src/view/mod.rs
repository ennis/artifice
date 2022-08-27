use crate::{
    eval::{
        imaging::{PxSizeI, RequestWindow, TiPoint, TiRect, TiSize},
        EvalState, Evaluation,
    },
    model::{metadata, Document},
};
use kyute::{
    cache, composable, graal,
    graal::{vk, Frame, PassBuilder, SubmitInfo},
    shell::{animation::Layer, application::Application, winit::window::WindowBuilder},
    style::Shape,
    text::FormattedTextExt,
    widget::{Retained, RetainedWidget, Text, WidgetExt},
    Alignment, Environment, Event, EventCtx, Geometry, LayoutCtx, LayoutParams, Measurements, PaintCtx, UnitExt,
    Widget, WidgetId, Window,
};
use kyute_common::{Atom, SizeI};
use std::fs;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Native vulkan view
////////////////////////////////////////////////////////////////////////////////////////////////////
pub struct NativeLayerWidget {
    image_id: Option<graal::ImageId>,
    image_handle: Option<vk::Image>,
    image_size: SizeI,
}

impl Drop for NativeLayerWidget {
    fn drop(&mut self) {
        if let Some(image_id) = self.image_id {
            let gpu_device = Application::instance().gpu_device();
            gpu_device.destroy_image(image_id);
        }
    }
}

impl NativeLayerWidget {
    /// Renders the current view.
    fn render(&mut self, layer: &Layer, scale_factor: f64) {
        if self.image_id.is_none() {
            panic!();
        }

        trace!("NativeLayerWidget::render");

        let image_id = self.image_id.unwrap();
        let image_handle = self.image_handle.unwrap();

        let mut gpu_context = Application::instance().lock_gpu_context();
        //let gpu_device = Application::instance().gpu_device();
        let layer_surface = layer.acquire_surface();
        let layer_image = layer_surface.image_info();

        let mut frame = Frame::new();

        let blit_w = self.image_size.width.min(layer.size().width);
        let blit_h = self.image_size.height.min(layer.size().height);

        frame.add_pass(
            PassBuilder::new()
                .name("blit to screen")
                .image_dependency(
                    image_id,
                    vk::AccessFlags::TRANSFER_READ,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                )
                .image_dependency(
                    layer_image.id,
                    vk::AccessFlags::TRANSFER_WRITE,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                )
                .record_callback(Box::new(move |context, _, command_buffer| {
                    let dst_image_handle = layer_image.handle;
                    let src_image_handle = image_handle;

                    let regions = &[vk::ImageBlit {
                        src_subresource: vk::ImageSubresourceLayers {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            mip_level: 0,
                            base_array_layer: 0,
                            layer_count: 1,
                        },
                        src_offsets: [
                            vk::Offset3D { x: 0, y: 0, z: 0 },
                            vk::Offset3D {
                                x: blit_w as i32,
                                y: blit_h as i32,
                                z: 1,
                            },
                        ],
                        dst_subresource: vk::ImageSubresourceLayers {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            mip_level: 0,
                            base_array_layer: 0,
                            layer_count: 1,
                        },
                        dst_offsets: [
                            vk::Offset3D { x: 0, y: 0, z: 0 },
                            vk::Offset3D {
                                x: blit_w as i32,
                                y: blit_h as i32,
                                z: 1,
                            },
                        ],
                    }];

                    unsafe {
                        context.vulkan_device().cmd_blit_image(
                            command_buffer,
                            src_image_handle,
                            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                            dst_image_handle,
                            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                            regions,
                            vk::Filter::NEAREST,
                        );
                    }
                })),
        );

        gpu_context.submit_frame(&mut (), frame, &SubmitInfo::default());
    }
}

impl RetainedWidget for NativeLayerWidget {
    type Args = Document;

    fn new(document: &Self::Args) -> Self {
        // find the display node
        let mut display_image = None;
        for node in document.root().children.values() {
            if let Some(op) = node.metadata(metadata::OPERATOR) {
                if &*op == "display" {
                    //display_node = Some(node.path.clone());
                    display_image = node
                        .attribute(&Atom::from("input:image"))
                        .and_then(|v| v.connection.clone())
                }
            }
        }

        let image_id;
        let image_handle;
        let image_size;

        if let Some(display_image) = display_image {
            // evaluate the input of the display node
            let device = Application::instance().gpu_device().clone();
            let eval = Evaluation::new(device, document.clone());
            let images = eval
                .device_evaluate_image(
                    &display_image,
                    0.0,
                    &RequestWindow {
                        roi: TiRect::new(TiPoint::zero(), TiSize::new(1280.0, 720.0)),
                        resolution: PxSizeI::new(1280, 720),
                    },
                )
                .unwrap();
            let image = images.planes.first().unwrap().1;
            image_id = Some(image.id);
            image_size = SizeI::new(image.size.width, image.size.height);
            image_handle = Some(image.handle);
        } else {
            image_id = None;
            image_handle = None;
            image_size = SizeI::default();
        }

        NativeLayerWidget {
            image_id,
            image_handle,
            image_size,
        }
    }

    fn update(&mut self, args: &Self::Args) {
        // nothing
    }

    fn widget_id(&self) -> Option<WidgetId> {
        None
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, params: &LayoutParams, env: &Environment) -> Geometry {
        Geometry {
            x_align: Alignment::CENTER,
            y_align: Alignment::CENTER,
            padding_left: 0.0,
            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
            measurements: Measurements::new(params.max),
        }
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &mut Event, env: &Environment) {
        // nothing to do
    }

    fn paint(&mut self, ctx: &mut PaintCtx) {
        // nothing to do (all done in layer_paint)
    }

    fn layer_paint(&mut self, ctx: &mut kyute::LayerPaintCtx, layer: &Layer, scale_factor: f64) {
        self.render(layer, scale_factor)
    }
}

#[composable]
fn document_window_contents(document: &Document) -> impl Widget {
    Retained::<NativeLayerWidget>::new(document)

    /*Text::new("-- NO SIGNAL --".font_size(40.0).font_family("MS 33558"))
    .centered()
    .frame(100.percent(), 100.percent())
    .background("rgb(10 10 10 / 255)", Shape::rectangle())*/
}

/// Native window displaying a document.
#[composable]
pub fn document_window(document: &Document) -> Window {
    Window::new(
        WindowBuilder::new().with_title("Document"),
        document_window_contents(document),
        None,
    )
}

fn try_open_document() -> anyhow::Result<Document> {
    let xml = fs::read_to_string("data/networks/simple.xml")?;
    let document = Document::from_xml(&xml)?;
    eprintln!("{:?}", document);
    Ok(document)
}

/// Application root.
#[composable]
pub fn application_root() -> impl Widget {
    let document_file_state = cache::state(|| Some(try_open_document().unwrap()));

    let mut doc = document_file_state.take_without_invalidation().unwrap();

    let rev = doc.revision;
    let window = document_window(&doc);
    if doc.revision != rev {
        document_file_state.set(Some(doc));
    } else {
        document_file_state.set_without_invalidation(Some(doc));
    }

    window
}
