use inline_spirv::{include_spirv, inline_spirv};
use lazy_static::lazy_static;
use crate::vk::ClearColorValue;
use graal::{vk, ImageResourceCreateInfo};
use mlr::{
    descriptor::{AttachmentLoadOp, AttachmentStoreOp},
    image::ImageAny,
};
use mlr::pipeline::{ColorBlendState, GraphicsPipeline, GraphicsPipelineBuilder};

static BACKGROUND_SHADER_VERT: &[u32] = include_spirv!("../../graal-bench/shaders/background.vert", vert);
static BACKGROUND_SHADER_FRAG: &[u32] = include_spirv!("../../graal-bench/shaders/background.frag", frag);

lazy_static! {
    static ref BACKGROUND_VERTEX_SHADER_MODULE: Shader = Shader::from_spirv_static(include_spirv!("../../graal-bench/shaders/background.vert", vert));
    static ref BACKGROUND_FRAGMENT_SHADER_MODULE: Shader = Shader::from_spirv_static(include_spirv!("../../graal-bench/shaders/background.frag", frag));
}

#[test]
fn test_image() {
    let device = graal::Device::new(None);
    let context = mlr::context::Context::new(device);

    let mut image = ImageAny::new(
        &context,
        graal::MemoryLocation::GpuOnly,
        ImageResourceCreateInfo {
            image_type: vk::ImageType::TYPE_2D,
            usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            format: vk::Format::R16G16B16A16_SFLOAT,
            extent: vk::Extent3D {
                width: 512,
                height: 512,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: 1,
            tiling: vk::ImageTiling::OPTIMAL,
        },
    );

    let mut fragment_output = mlr::descriptor::FragmentOutput::builder()
        .add_color_attachment(
            &mut image,
            AttachmentLoadOp::Clear {
                value: vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.2, 0.2, 0.8, 1.0],
                    },
                },
            },
            AttachmentStoreOp::Store,
        )
        .build();

    // can build on-the-fly, or stash it somewhere, it doesn't matter
    let pipeline =
        GraphicsPipeline::builder()
            .with_color_blend_state(ColorBlendState {
                logic_op: None,
                attachments: (),
                blend_constants: []
            })
            
            .build();




    draw! {

        vertex_input { forall vertex_buffers }

        input_assembly {
            indices: <index buffer>,
            topology: Triangle,
        }

        vertex_shader { BACKGROUND_VERTEX_SHADER_MODULE }

        rasterization {
            depth_clamp_enable: false,
            polygon_mode: PolygonMode::Fill,
            cull_mode: CullModeFlags::NONE,
            depth_bias: DepthBias::Disabled,
            front_face: FrontFace::Clockwise,
            line_width: 1.0f32,
        }

        viewport { 0: { width: target_size.0 as f32,
                        height: target_size.1 as f32,
                        min_depth: 0.0,
                        max_depth: 1.0 } }

        scissor { 0: { enabled: false } }

        fragment_shader { BACKGROUND_FRAGMENT_SHADER_MODULE }

        blending {
             0: {
                target: &mut image,
                color_blend_op: BlendOp::Add,
                src_color_blend_factor: BlendFactor::SrcAlpha,
                dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
                alpha_blend_op: BlendOp::Add,
                src_alpha_blend_factor: BlendFactor::One,
                dst_alpha_blend_factor: BlendFactor::Zero,
                color_write_mask: ColorComponentFlags::ALL,
            }
        }

        logic_ops {
            0: {

            }
        }
    };


}
