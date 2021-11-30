use mlr::shader::ShaderArguments;

#[derive(Copy, Clone, Debug)]
#[derive(mlr::ShaderArguments)]
#[repr(C)]
struct PerObjectData {
    resolution: [f32; 2],
    scroll_offset: [f32; 2],
    zoom: f32,
}

#[derive(mlr::ShaderArguments)]
#[repr(C)]
struct MaterialArguments {
    u_color: Vec4,
    #[argument(binding=1)] t_color: mlr::frame::SampledImage,
}


#[test]
fn test_descriptor() {
    //eprintln!("GlobalResources: {:#?}", GlobalResources::DESC);
    //eprintln!("PerObjectResources: {:#?}", PerObjectResources::DESC);
}
