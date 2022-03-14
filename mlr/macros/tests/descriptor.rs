use mlr::Arguments;
use std::{marker::PhantomData, mem};

/*#[derive(Copy, Clone, Debug)]
#[derive(mlr::ShaderArguments)]
#[repr(C)]
struct PerObjectData {
    resolution: [f32; 2],
    scroll_offset: [f32; 2],
    zoom: f32,
}*/

#[derive(mlr::Arguments)]
#[repr(C)]
struct MaterialArguments<'a> {
    u_color: [f32; 4],
    #[argument(binding = 1, stages(vertex, fragment))]
    t_color: mlr::SampledImage2D<'a>,
}

#[repr(C)]
struct PerObjectData {
    resolution: [f32; 2],
    scroll_offset: [f32; 2],
    zoom: f32,
}

#[test]
fn test_descriptor() {
    //eprintln!("GlobalResources: {:#?}", GlobalResources::DESC);
    //eprintln!("PerObjectResources: {:#?}", PerObjectResources::DESC);
}
