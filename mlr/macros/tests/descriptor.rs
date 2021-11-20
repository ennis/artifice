use mlr::shader::ShaderArguments;

#[derive(Copy, Clone, Debug)]
#[derive(ShaderArguments)]
#[repr(C)]
struct PerObjectData {
    resolution: [f32; 2],
    scroll_offset: [f32; 2],
    zoom: f32,
}

#[derive(ShaderArguments)]
#[repr(C)]
struct GlobalResources<'a> {
    #[argument(binding = 0, sampled_image, runtime_array(max_count = 1024))]
    textures: &'a [vk::DescriptorImageInfo],
}

#[derive(ShaderArguments)]
#[repr(C)]
struct PerObjectResources {
    #[argument(binding = 0, uniform_buffer, stages(all_graphics))]
    uniforms: vk::DescriptorBufferInfo,
    #[argument(binding = 1, uniform_buffer)]
    buffer: vk::DescriptorBufferInfo,
    #[argument(binding = 2, sampler, array)]
    samplers: [vk::Sampler; 4],
}

#[test]
fn test_descriptor() {
    eprintln!("GlobalResources: {:#?}", GlobalResources::DESC);
    eprintln!("PerObjectResources: {:#?}", PerObjectResources::DESC);
}
