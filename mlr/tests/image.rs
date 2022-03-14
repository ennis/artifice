use crate::vk::ClearColorValue;
use graal::{
    descriptor::{AttachmentLoadOp, AttachmentStoreOp, ColorAttachment, CombinedImageSampler2D},
    vk, ImageResourceCreateInfo,
};
use inline_spirv::{include_spirv, inline_spirv};
use lazy_static::lazy_static;
use mlr::{
    image::ImageAny,
    shader::{ArgumentBlock, ShaderModule},
    SampledImage2D,
};
use mlr::pipeline::PipelineConfig;

lazy_static! {
    static ref BACKGROUND_VERTEX_SHADER_MODULE: Shader =
        Shader::from_spirv_static(include_spirv!("../graal-bench/shaders/background.vert", vert));
    static ref BACKGROUND_FRAGMENT_SHADER_MODULE: Shader =
        Shader::from_spirv_static(include_spirv!("../graal-bench/shaders/background.frag", frag));
}

#[test]
fn test_image() {
    let device = graal::Device::new(None);
    let mut context = mlr::context::Context::new(device);
    let mut frame = context.start_frame();

    {
        let image = ImageAny::new(
            frame.device(),
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

        frame.submit_pass("draw_to_image", |pass| {
            let color_attachment = ColorAttachment::new(pass, &image);

            move |ctx| {
                // TODO draw
                color_attachment;
            }
        });
    }

    frame.finish();
}

// The function is the pipeline.
// -> the pipeline is "polymorphic", holds a cache of concrete pipelines that match
//    the parameters
#[mlr::pipeline_interface]
fn draw_item<T>(
    frame: &mlr::Frame,
    pipeline: &mlr::Pipeline,
    #[viewport] viewport: Viewport,
    #[attachment] color: ColorAttachment,
    #[attachment] normal: ColorAttachment,
    #[attachment] tangent: ColorAttachment,
    #[attachment] depth: DepthAttachment,
    #[argument(set = 0)] scene_args: &SceneArguments,
    #[argument(set = 1)] material_args: &MaterialArguments,
    #[vertex(binding = 0, location = 0, per_vertex)] vertices: VertexBufferView<Vertex>,
    #[vertex(binding = 2, location = 3, per_vertex)] previous_vertices: VertexBufferView<Vertex>,
) {
    // auto-generated
    pipeline_impl!(frame, pipeline
        COLOR_ATTACHMENT color, normal, tangent

        ARGUMENTS 0, SceneArguments, scene_args, 1, MaterialArguments, material_args
        VERTEX 0, 0, PER_VERTEX, Vertex, vertices, 2, 3, PER_VERTEX, Vertex, previous_vertices
    )

    // get matching pipeline from collection, given:
    // * static argument interfaces:
    //      - SceneArguments
    //      - MaterialArguments
    // * color attachment formats:
    //      - color, normal, tangent, depth
    // * vertex input formats:
    //      - vertices, previous_vertices

    //pipeline_collection

    device.create_pipeline::<mlr::ShaderInterface![draw_item]>(vertex, fragment, )

    mlr::create_pipeline!(vertex, fragment, draw_item<>)
}

#[mlr::pipeline_interface]
pub trait DrawItem {

    fn draw(frame: &mut mlr::Frame,
            #[viewport] viewport: Viewport,
            #[attachment] color: ColorAttachment,
            #[attachment] normal: ColorAttachment,
            #[attachment] tangent: ColorAttachment,
            #[attachment] depth: DepthAttachment,
            #[argument(set = 0)] scene_args: &SceneArguments,
            #[argument(set = 1)] material_args: &MaterialArguments,
            #[vertex(binding = 0, location = 0, per_vertex)] vertices: VertexBufferView<Vertex>,
            #[vertex(binding = 2, location = 3, per_vertex)] previous_vertices: VertexBufferView<Vertex>,)
}

#[mlr::pipeline]
#[pipeline(interface=DrawItem)]
#[pipeline(vertex_source_file="../")]
pub struct StaticDrawItem;

pub struct Dynamic_DrawItem {
    pipeline: mlr::Pipeline,
}

impl dyn DrawItem {
    pub fn new() -> Arc<dyn DrawItem> {
        // does stuff, verifies the interface



    }
}

// If the source is known statically, then resolves to a type that impls DrawItem
// Otherwise, returns a dyn DrawItem

#[test]
fn test_scene() {

    Blur.draw(...);

    let pipeline = DrawItem::new(
        PipelineConfig {
            vertex_shader:
        }
    );

    #[derive(mlr::Arguments)]
    #[repr(C)]
    struct SceneArguments {
        // uniform variables will be put in a single uniform buffer, at location 0
        u_view_matrix: Mat4,
        u_proj_matrix: Mat4,
        u_view_proj_matrix: Mat4,
        u_inverse_proj_matrix: Mat4,
    }

    #[derive(mlr::Arguments)]
    #[repr(C)]
    struct MaterialArguments<'a> {
        u_color: Vec4,
        #[argument(binding = 1)]
        t_color: SampledImage<'a>,
        #[argument(binding = 2)]
        t_specular: SampledImage<'a>,
    }

    let scene_args = SceneArguments {
        u_view_matrix: (),
        u_proj_matrix: (),
        u_view_proj_matrix: (),
        u_inverse_proj_matrix: (),
    };

    draw_item(frame, pipeline, &scene_args, &material_args, vertices);
}
