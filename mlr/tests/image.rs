use crate::vk::ClearColorValue;
use graal::{vk, ImageResourceCreateInfo};
use inline_spirv::{include_spirv, inline_spirv};
use lazy_static::lazy_static;
use mlr::{
    descriptor::{AttachmentLoadOp, AttachmentStoreOp, ColorAttachment},
    image::ImageAny,
    shader::ArgumentBlock,
    shader::Shader
};

lazy_static! {
    static ref BACKGROUND_VERTEX_SHADER_MODULE: Shader = Shader::from_spirv_static(include_spirv!(
        "../graal-bench/shaders/background.vert",
        vert
    ));
    static ref BACKGROUND_FRAGMENT_SHADER_MODULE: Shader = Shader::from_spirv_static(
        include_spirv!("../graal-bench/shaders/background.frag", frag)
    );
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

        frame.submit_pass("draw_to_image", |pass_builder| {
            let color_attachment = ColorAttachment::new(pass_builder, &image);
            move |ctx| {
                // TODO draw
                color_attachment;
            }
        });
    }

    frame.finish();
}

/*#[test]
fn test_scene() {
    #[derive(mlr::ShaderArguments)]
    #[repr(C)]
    struct SceneArguments {
        // uniform variables will be put in a single uniform buffer, at location 0
        u_view_matrix: Mat4,
        u_proj_matrix: Mat4,
        u_view_proj_matrix: Mat4,
        u_inverse_proj_matrix: Mat4,
    }

    #[derive(mlr::ShaderArguments)]
    #[repr(C)]
    struct MaterialArguments {
        u_color: Vec4,
        #[argument(binding=1)] t_color: SampledImage,
    }

    ctx.submit_pass(|pass| {
        // split borrows will help here
        let target = ColorAttachment::new(pass, &background_image);

        move |ctx| {
            let scene_args = ArgumentBlock::new(SceneArguments {
                u_view_matrix: (),
                u_proj_matrix: (),
                u_view_proj_matrix: (),
                u_inverse_proj_matrix: ()
            });

            for batch in material_batches.iter() {
                let material_args = ArgumentBlock::new(
                    MaterialArguments {
                        u_color: (),
                        t_color: TextureDescriptor::new(&batch.texture, Sampler::linear())
                    });

                for mesh in batch.objects.iter() {
                    // issue: validation that batch.texture is in the correct state here.
                    ctx.draw(&[&scene_args, &material_args])
                }
            }
        }
    });


    // alternatively: return an fn
    // see https://media.contentapi.ea.com/content/dam/ea/seed/presentations/wihlidal-halcyonarchitecture-notes.pdf
}*/
