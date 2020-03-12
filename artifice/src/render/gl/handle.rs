use crate::render::gl::api::Gl;
use crate::render::gl::api::gl::types::*;

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

impl_handle_type!(pub struct TextureHandle(GLuint));
impl_handle_type!(pub struct RenderbufferHandle(GLuint));
impl_handle_type!(pub struct FramebufferHandle(GLuint));
impl_handle_type!(pub struct VertexArrayHandle(GLuint));
impl_handle_type!(pub struct BufferHandle(GLuint));
impl_handle_type!(pub struct ProgramHandle(GLuint));

impl_handle_type!(pub struct VertexShaderHandle(GLuint));
impl_handle_type!(pub struct FragmentShaderHandle(GLuint));

impl Drop for TextureHandle {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteTextures(1, &mut self.obj)
        }
    }
}

impl Drop for RenderbufferHandle {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteRenderbuffers(1, &mut self.obj)
        }
    }
}

impl Drop for FramebufferHandle {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteFramebuffers(1, &mut self.obj)
        }
    }
}

impl Drop for VertexArrayHandle {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteVertexArrays(1, &mut self.obj)
        }
    }
}

impl Drop for BufferHandle {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteBuffers(1, &mut self.obj)
        }
    }
}

impl Drop for VertexShaderHandle {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteShader(self.obj)
        }
    }
}

impl Drop for FragmentShaderHandle {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteShader(self.obj)
        }
    }
}

impl Drop for ProgramHandle {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteProgram(self.obj)
        }
    }
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

