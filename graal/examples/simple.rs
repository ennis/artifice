use graal::vk;
use raw_window_handle::HasRawWindowHandle;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn create_test_image(context: &mut graal::Context, name: &str) -> graal::ResourceId {
    context.create_image_resource(
        name,
        graal::vk::ImageType::TYPE_2D,
        graal::vk::ImageUsageFlags::COLOR_ATTACHMENT
            | graal::vk::ImageUsageFlags::SAMPLED
            | graal::vk::ImageUsageFlags::TRANSFER_DST,
        graal::vk::Format::R8G8B8A8_SRGB,
        graal::vk::Extent3D {
            width: 1280,
            height: 720,
            depth: 1,
        },
        graal::vk::MemoryPropertyFlags::DEVICE_LOCAL,
        graal::vk::MemoryPropertyFlags::DEVICE_LOCAL,
        &graal::ImageProperties {
            mip_levels: 1,
            array_layers: 1,
            samples: 1,
            ..Default::default()
        },
    )
}

fn test_pass(
    batch: &mut graal::Batch,
    name: &str,
    images: &[(
        graal::ResourceId,
        graal::vk::AccessFlags,
        graal::vk::PipelineStageFlags,
        graal::vk::PipelineStageFlags,
        graal::vk::ImageLayout,
    )],
) {
    let mut pass_builder = batch.build_render_pass(name);
    for &(img, access_mask, input_stage, output_stage, layout) in images {
        pass_builder.add_image_usage(img, access_mask, input_stage, output_stage, layout);
    }
    pass_builder.finish();
}

fn color_attachment_output(
    img: graal::ResourceId,
) -> (
    graal::ResourceId,
    vk::AccessFlags,
    vk::PipelineStageFlags,
    vk::PipelineStageFlags,
    vk::ImageLayout,
) {
    (
        img,
        vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    )
}

fn sample_image(
    img: graal::ResourceId,
) -> (
    graal::ResourceId,
    vk::AccessFlags,
    vk::PipelineStageFlags,
    vk::PipelineStageFlags,
    vk::ImageLayout,
) {
    (
        img,
        vk::AccessFlags::SHADER_READ,
        vk::PipelineStageFlags::VERTEX_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER,
        Default::default(),
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    )
}

fn compute_read(
    img: graal::ResourceId,
) -> (
    graal::ResourceId,
    vk::AccessFlags,
    vk::PipelineStageFlags,
    vk::PipelineStageFlags,
    vk::ImageLayout,
) {
    (
        img,
        vk::AccessFlags::SHADER_READ,
        vk::PipelineStageFlags::COMPUTE_SHADER,
        Default::default(),
        vk::ImageLayout::GENERAL,
    )
}

fn compute_write(
    img: graal::ResourceId,
) -> (
    graal::ResourceId,
    vk::AccessFlags,
    vk::PipelineStageFlags,
    vk::PipelineStageFlags,
    vk::ImageLayout,
) {
    (
        img,
        vk::AccessFlags::SHADER_WRITE,
        vk::PipelineStageFlags::COMPUTE_SHADER,
        vk::PipelineStageFlags::COMPUTE_SHADER,
        vk::ImageLayout::GENERAL,
    )
}

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let surface = graal::surface::get_vulkan_surface(window.raw_window_handle());
    let device = graal::Device::new(surface);
    let mut context = graal::Context::new(device);

    let img_a = create_test_image(&mut context, "A");
    let img_b = create_test_image(&mut context, "B");
    let img_c = create_test_image(&mut context, "C");
    let img_d1 = create_test_image(&mut context, "D1");
    let img_d2 = create_test_image(&mut context, "D2");
    let img_e = create_test_image(&mut context, "E");
    let img_f = create_test_image(&mut context, "F");
    let img_g = create_test_image(&mut context, "G");
    let img_h = create_test_image(&mut context, "H");
    let img_i = create_test_image(&mut context, "I");
    let img_j = create_test_image(&mut context, "J");
    let img_k = create_test_image(&mut context, "K");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                println!("The close button was pressed; stopping");
                *control_flow = ControlFlow::Exit
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let mut batch = context.start_batch();

                test_pass(&mut batch, "P0", &[color_attachment_output(img_a)]);
                test_pass(&mut batch, "P1", &[color_attachment_output(img_b)]);
                test_pass(
                    &mut batch,
                    "P2",
                    &[
                        compute_read(img_a),
                        compute_read(img_b),
                        compute_write(img_d1),
                        compute_write(img_d2),
                    ],
                );
                test_pass(&mut batch, "P3", &[color_attachment_output(img_c)]);
                test_pass(
                    &mut batch,
                    "P4",
                    &[
                        compute_read(img_d2),
                        compute_read(img_c),
                        compute_write(img_e),
                    ],
                );
                test_pass(
                    &mut batch,
                    "P5",
                    &[compute_read(img_d1), compute_write(img_f)],
                );
                test_pass(
                    &mut batch,
                    "P6",
                    &[
                        compute_read(img_e),
                        compute_read(img_f),
                        compute_write(img_g),
                    ],
                );
                test_pass(
                    &mut batch,
                    "P7",
                    &[compute_read(img_g), compute_write(img_h)],
                );
                test_pass(
                    &mut batch,
                    "P8",
                    &[compute_read(img_h), compute_write(img_i)],
                );
                test_pass(
                    &mut batch,
                    "P9",
                    &[
                        compute_read(img_i),
                        compute_read(img_g),
                        compute_write(img_j),
                    ],
                );
                test_pass(
                    &mut batch,
                    "P10",
                    &[compute_read(img_j), compute_write(img_k)],
                );

                batch.finish();
            }
            _ => (),
        }
    });
}
