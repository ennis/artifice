use std::sync::Arc;
use graal::vk;
use mlr::{
    pipeline::{GraphicsPipelineBuilder, RG16Float, RGBA16Float, RGBA8},
    vertex::Norm,
    GraphicsPipelineConfig,
};



#[test]
fn test_scene() {

    let mut b = VertexInputLayoutBuilder::new();
    b.push_attribute(0, )

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
}
