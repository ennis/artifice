use crate::api::gl::types::*;
use crate::api::Gl;

pub struct ContextObject<T> {
    gl: Gl,
    obj: T,
}

macro_rules! impl_handle_type {
    ($v:vis struct $name:ident($obj:ty)) => {
        $v struct $name {
            pub gl: Gl,
            pub obj: $obj,
        }

        impl $name {
            $v unsafe fn from_raw(gl: &Gl, obj: $obj) -> $name {
                $name {
                    gl: gl.clone(),
                    obj
                }
            }
        }
    };
}

// strategy for the rest:
// - safe draw()
//      - Uniforms::set(UniformCtx)
// - UniformCtx to set uniforms (by name, index, whatever)
//      - also uniform buffers
//
// Framebuffers?
// - cache whenever appropriate (FramebufferHandle)
// - impl Into<Framebuffer> for [FramebufferAttachment]
// - has meta information to check against interface
//
// Programs?
// - in the future, reflection info
//
// Stateless layer?
// - struct
//
// Textures?
// - easy data upload + readback (Texture1D, Texture2D methods)
