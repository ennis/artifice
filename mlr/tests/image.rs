use inline_spirv::{include_spirv, inline_spirv};
use lazy_static::lazy_static;
use crate::vk::ClearColorValue;
use graal::{vk, ImageResourceCreateInfo};
use mlr::{
    descriptor::{AttachmentLoadOp, AttachmentStoreOp},
    image::ImageAny,
};
use mlr::frame::{ColorAttachment, PassBuilder, PassSubmitCtx, SampledImage};
use mlr::pipeline::{ColorBlendState, GraphicsPipeline, GraphicsPipelineBuilder};
use mlr::shader::ArgumentBlock;

static BACKGROUND_SHADER_VERT: &[u32] = include_spirv!("../../graal-bench/shaders/background.vert", vert);
static BACKGROUND_SHADER_FRAG: &[u32] = include_spirv!("../../graal-bench/shaders/background.frag", frag);

lazy_static! {
    static ref BACKGROUND_VERTEX_SHADER_MODULE: Shader = Shader::from_spirv_static(include_spirv!("../../graal-bench/shaders/background.vert", vert));
    static ref BACKGROUND_FRAGMENT_SHADER_MODULE: Shader = Shader::from_spirv_static(include_spirv!("../../graal-bench/shaders/background.frag", frag));
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


        // `mlr::Pass` returns an anonymous `impl Pass + 'a`.
        let pass = mlr::pass! {
            [background_image as mlr::ColorAttachment, group]
            move |ctx| {
                // ... background_image is accessible here, as an object of type `mlr::ColorAttachment`.
                // can borrow stuff as long as it lives for the duration of the frame
                // what about resource groups?
                // resource groups usually live for the duration of the frame, so they can be borrowed
                // plus, when you "freeze" a resource in some state, you get the mlr::ColorAttachment or mlr::Texture that you need.

                ctx.draw(...);
            }
        };




        /*frame.submit(MyBackgroundPass {
            // and then you are fucked, because the image only lives for the scope starting at L27.
            // pass objects cannot reference any local data.
            // -> view objects? what about resource groups?
            //
            // Options:
            // - the pass object doesn't borrow the resource
            //      - but then it may be deleted before it is registered as a dependency
            // - the pass object borrows, but during setup, create a `PassResources` object.
            //      - X3 verbosity
            //      - automatically derive the pass resources?
            //          - now it's invisible to autocomplete...
            // - Refcounted resources
            //      - prevents marking them as unused until finalization of the frame, and thus prevents aliasing: NOPE
            //
            // FUCK THIS SHIT
            //
            // - Big ol' f*cking macro.
            //
            background_image: &image
        })*/
    }

}

#[test]
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
}