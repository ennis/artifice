use graal::{swapchain::Swapchain, Device};
use mlr::{
    context::RenderPassDescriptor,
    pipeline::{GraphicsPipeline, PipelineLayout, PipelineLayoutDescriptor},
    sampler::Linear_ClampToEdge,
    vk, AttachmentLoadOp, AttachmentStoreOp, ContextResources, RenderPassColorAttachment,
};
use raw_window_handle::HasRawWindowHandle;
use std::sync::Arc;
use winit::{
    event::{Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

#[derive(Copy, Clone, Debug, mlr::Arguments)]
#[repr(C)]
struct BackgroundParams {
    u_resolution: [f32; 2],
    u_scroll_offset: [f32; 2],
    u_zoom: f32,
}

// API for creating non-frame-bound resources?
//
// RenderPass derefs to Frame derefs to Context?
//
// In the end, we have to do renderpass.create_sampler(), which feels weird
// worryingly, we can also do `renderpass.start_render_pass()` since renderpass derefs to frame, which
// is definitely not valid =>
//
//
// You got:
// - Frame, which owns FrameResources, which borrows ContextResources
// - Context, which owns ContextResources
// - RenderPass, which borrows Frame
//
// RenderPass derefs to FrameResources, which then derefs to ContextResources
// Frame derefs to FrameResources, which derefs to ContextResources
// Context derefs to ContextResources
//
// Alternative:
// - mlr::Device to allocate most of the stuff
// - mlr::Context to create frames
// - mlr::Frame to build a frame
//
// mlr::Device is not borrowed by anything and can be used at any time to create resources
// However, since mlr::Frame allocates transient resources & objects (buffers, samplers, image views...),
// it also should have access to the device internally, so there must be some sharing (via `Arc<Device>`) occurring.
//
// The first option doesn't have that limitation.

fn create_pipeline(device: &mut mlr::Device) -> GraphicsPipeline {
    let layouts = &[device.get_or_create_descriptor_set_layout_for_type::<BackgroundParams>()];

    let pipeline_layout = PipelineLayout::new(ctx.device(), &PipelineLayoutDescriptor { layouts });

    //BackgroundParams::DESCRIPTOR_SET_LAYOUT_BINDINGS
}

fn draw_frame(device: &mut mlr::Device, frame: &mut mlr::Frame) {
    let target = device.create_image(
        graal::MemoryLocation::GpuOnly,
        graal::ImageResourceCreateInfo {
            image_type: vk::ImageType::TYPE_2D,
            usage: vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            format: vk::Format::R16G16B16A16_SFLOAT,
            extent: vk::Extent3D {
                width: 1024,
                height: 1024,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: 1,
            tiling: Default::default(),
        },
    );

    let mut render_pass = frame.start_render_pass(&RenderPassDescriptor {
        color_attachments: &[RenderPassColorAttachment {
            attachment: &target,
            load_op: AttachmentLoadOp::Clear {
                value: [0.0, 0.2, 0.4, 1.0],
            },
            store_op: AttachmentStoreOp::Store,
        }],
        depth_stencil_attachment: None,
    });

    let params = device.create_argument_block(BackgroundParams {
        u_resolution: [1024.0, 1024.0],
        u_scroll_offset: [0.0, 0.0],
        u_zoom: 1.0,
    });

    render_pass.draw(
        pipeline,
        MyPipelineArguments {
            // vertices ...
            params,
            //
        },
    );

    render_pass.finish();
}

fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::ACTIVE)
        .init();

    // Ancient mantra of window and context creation
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let mut swapchain_size: (u32, u32) = window.inner_size().into();
    let surface = graal::surface::get_vulkan_surface(window.raw_window_handle());

    let (device, mut context) = unsafe { mlr::create_device_and_context(Some(surface)) };

    let mut swapchain =
        unsafe { Swapchain::new(device.backend(), surface, window.inner_size().into()) };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { window_id, event } => match event {
                WindowEvent::CloseRequested => {
                    println!("The close button was pressed; stopping");
                    *control_flow = ControlFlow::Exit
                }
                WindowEvent::Resized(size) => unsafe {
                    swapchain_size = size.into();
                    eprintln!("window resized: {},{}", swapchain_size.0, swapchain_size.1);
                    swapchain.resize(&context.device(), swapchain_size);
                },
                _ => {}
            },
            Event::MainEventsCleared => {
                window.request_redraw();
            }

            Event::RedrawRequested(_) => {
                let frame = context.start_frame();
            }

            _ => (),
        }
    });
}
