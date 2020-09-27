use crate::texture::TextureHandle;
use crate::buffer::BufferHandle;
use crate::api::{Gl, gl};
use crate::api::gl::types::GLuint;
use std::ffi::CString;
use artifice_gfxbase::sampling::SamplerDescription;
use crate::sampler_cache::SamplerCache;
use crate::framebuffer::FramebufferHandle;
use crate::shader::ProgramHandle;

pub enum Uniform<'a> {
    U1i(i32),
    U2iv(&'a [i32;2]),
    U3iv(&'a [i32;3]),
    U4iv(&'a [i32;4]),
    U1f(f32),
    U2fv(&'a [f32;2]),
    U3fv(&'a [f32;3]),
    U4fv(&'a [f32;3]),
    UMatrix4fv(&'a [f32; 16]),
    UMatrix4fvTranspose(&'a [f32; 16]),
}

pub trait ShaderResourceBuilder {
    fn uniform1i(&mut self, n: &str, v: i32);
    fn uniform2iv(&mut self, n: &str, v: &[i32;2]);
    fn uniform3iv(&mut self, n: &str, v: &[i32;3]);
    fn uniform4iv(&mut self, n: &str, v: &[i32;4]);

    fn uniform1f(&mut self, n: &str, v: f32);
    fn uniform2fv(&mut self, n: &str, v: &[f32;2]);
    fn uniform3fv(&mut self, n: &str, v: &[f32;3]);
    fn uniform4fv(&mut self, n: &str, v: &[f32;4]);

    fn uniform_matrix_4fv(&mut self, n: &str, v: &[[f32;4];4]);
    fn uniform_matrix_4fv_transpose(&mut self, n: &str, v: &[[f32;4];4]);

    fn texture(&mut self, unit: u32, tex: &TextureHandle);
    fn texture_sampler(&mut self, unit: u32, tex: &TextureHandle, sampler: &SamplerDescription);

    fn image(&mut self, unit: u32, tex: &TextureHandle);
}

pub trait RenderStateBuilder {
    fn viewport(&mut self, x: i32, y: i32, w: i32, h: i32);
    fn scissor(&mut self, x: i32, y: i32, w: i32, h: i32);
    fn depth_test(&mut self, t: DepthTest);
}

struct DrawResourceBuilder<'a> {
    gl: &'a Gl,
    program: GLuint,
    sampler_cache: &'a mut SamplerCache,
}

impl DrawResourceBuilder {
    fn get_uniform_location(&self, n: &str) -> i32 {
        unsafe {
            self.gl.GetUniformLocation(self.program, CString::from(n).as_ptr())
        }
    }
}

impl ShaderResourceBuilder for DrawResourceBuilder {
    fn uniform1i(&mut self, n: &str, v: i32) {
        unsafe {
            let loc = self.get_uniform_location(n);
            self.gl.Uniform1i(loc, v);
        }
    }

    fn uniform2iv(&mut self, n: &str, v: &[i32; 2]) {
        unsafe {
            let loc = self.get_uniform_location(n);
            self.gl.Uniform2iv(loc, 1,v.as_ptr());
        }
    }

    fn uniform3iv(&mut self, n: &str, v: &[i32; 3]) {
        unsafe {
            let loc = self.get_uniform_location(n);
            self.gl.Uniform3iv(loc, 1,v.as_ptr());
        }
    }

    fn uniform4iv(&mut self, n: &str, v: &[i32; 4]) {
        unsafe {
            let loc = self.get_uniform_location(n);
            self.gl.Uniform4iv(loc, 1,v.as_ptr());
        }
    }

    fn uniform1f(&mut self, n: &str, v: f32) {
        unsafe {
            let loc = self.get_uniform_location(n);
            self.gl.Uniform1f(loc, v);
        }
    }

    fn uniform2fv(&mut self, n: &str, v: &[f32; 2]) {
        unsafe {
            let loc = self.get_uniform_location(n);
            self.gl.Uniform4fv(loc, 1,v.as_ptr());
        }
    }

    fn uniform3fv(&mut self, n: &str, v: &[f32; 3]) {
        unsafe {
            let loc = self.get_uniform_location(n);
            self.gl.Uniform4fv(loc, 1,v.as_ptr());
        }
    }

    fn uniform4fv(&mut self, n: &str, v: &[f32; 4]) {
        unsafe {
            let loc = self.get_uniform_location(n);
            self.gl.Uniform4fv(loc, 1,v.as_ptr());
        }
    }

    fn uniform_matrix_4fv(&mut self, n: &str, v: &[[f32; 4]; 4]) {
        unsafe {
            let loc = self.get_uniform_location(n);
            self.gl.UniformMatrix4fv(loc, 1, gl::FALSE,v.as_ptr() as *const _);
        }
    }

    fn uniform_matrix_4fv_transpose(&mut self, n: &str, v: &[[f32; 4]; 4]) {
        unsafe {
            let loc = self.get_uniform_location(n);
            self.gl.UniformMatrix4fv(loc, 1, gl::TRUE,v.as_ptr() as *const _);
        }
    }

    fn texture(&mut self, unit: u32, tex: &TextureHandle) {
        unsafe {
            self.gl.BindTextureUnit(unit, tex.obj);
        }
    }

    fn texture_sampler(&mut self, unit: u32, tex: &TextureHandle, sampler: &SamplerDescription) {
        unsafe {
            self.gl.BindTextureUnit(unit, tex.obj);
            self.gl.BindSampler(unit, self.sampler_cache.get_sampler(&self.gl, sampler).obj);
        }
    }

    fn image(&mut self, unit: u32, tex: &TextureHandle) {
        unsafe {
            self.gl.BindImageTextures(unit, 1, (&[tex.obj]).as_ptr())
        }
    }
}

// mesh source: a type
// -> bind(gl,vao cache): binds everything (vertex+index buffers+vao)
// -> draw_params(): returns the draw params

pub fn draw<U: Fn(&mut DrawResourceBuilder)>(
    gl: &Gl,
    framebuffer: &FramebufferHandle,
    program: &ProgramHandle,
    shader_resources: U
)
{
    let mut r = DrawResourceBuilder {
        gl,
        program: program.obj,
        sampler_cache: unimplemented!()
    };

    shader_resources(&mut r);

}