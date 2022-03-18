use graal_spirv::spv::Scope::Device;
use mlr::{
    pipeline::{GraphicsPipelineBuilder, RG16Float, RGBA16Float, RGBA8},
    vertex::Norm,
    GraphicsPipelineConfig,
};

#[repr(C)]
#[derive(VertexData, Copy, Clone)]
struct Vertex {
    pos: [f32; 3],
    norm: [f32; 3],
    tex: [Norm<u16>; 2],
}

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

// the problem with PipelineInterfaces is that all parameters don't have the same variability:
//
#[derive(mlr::PipelineInterface)]
struct DrawPipelineInterface<'a> {
    #[vertex_input]
    vertex_input: (VertexBufferView<Vertex>, VertexBufferView<Vertex>),

    // can specify shader arguments inline
    u_color: Vec4,
    #[argument(binding = 1)]
    t_color: SampledImage<'a>,
    #[argument(binding = 2)]
    t_specular: SampledImage<'a>,

    /// Color buffer, R16G16B16A16_SFLOAT
    #[color_attachment(
        color,
        samples = 1,
        load_op = "CLEAR",
        store_op = "STORE",
        layout = "COLOR_ATTACHMENT_OPTIMAL"
    )]
    color: graal::ImageInfo,
    /// Normals, RG16_SFLOAT
    #[color_attachment(
        color,
        samples = 1,
        load_op = "CLEAR",
        store_op = "STORE",
        layout = "COLOR_ATTACHMENT_OPTIMAL"
    )]
    normal: graal::ImageInfo,
    /// Tangents: RG16_SFLOAT
    #[color_attachment(
        color,
        samples = 1,
        load_op = "CLEAR",
        store_op = "STORE",
        layout = "COLOR_ATTACHMENT_OPTIMAL"
    )]
    tangent: graal::ImageInfo,

    /// Depth: D32_SFLOAT
    #[attachment(
        depth,
        samples = 1,
        load_op = "CLEAR",
        store_op = "STORE",
        layout = "DEPTH_STENCIL_ATTACHMENT_OPTIMAL"
    )]
    depth: graal::ImageInfo,

    #[viewport]
    viewport: Viewport,
}

#[test]
fn test_scene() {
    let device = GraphicsPipelineBuilder::new()
        .with_vertex_input::<(Vertex, Vertex)>()
        .with_fragment_output::<(RGBA16Float, RGBA16Float, RG16Float, RGBA8)>()
        .build(&GraphicsPipelineConfig {
            vertex_shader: &(),
            fragment_shader: &(),
            primitive_state: PrimitiveState {},
            multisample_state: MultisampleState {},
            depth_stencil_state: None,
            color_attachments: &[],
        });

    /* create_pipeline!(device, config,
        VERTEX_INPUT [ VertexBufferView<MyVertex> ]
        ARGUMENTS    [ SceneArguments, MaterialArguments ]
        FRAGMENT_OUTPUT COLOR [ R16G16B16A16_SFLOAT ] DEPTH [ None ] STENCIL [ None ]
    )*/

    /*Blur.draw(...);

    let pipeline = DrawItem::new(
        PipelineConfig {
            vertex_shader:
        }
    );


    draw_item(frame, pipeline, &scene_args, &material_args, vertices);*/
}
