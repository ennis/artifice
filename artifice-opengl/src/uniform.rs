use std::ffi::CString;
use crate::api::Gl;
use crate::shader::ProgramHandle;
use crate::texture::TextureHandle;

/// Interface for setting uniforms
struct UniformCtx<'a> {
    gl: Gl,
    prog: &'a ProgramHandle,
}

// TODO might not need to be unsafe
unsafe fn get_uniform_location(prog: &ProgramHandle, name: &str) -> i32 {
    let name = CString::new(name).unwrap();
    prog.gl.GetUniformLocation(prog.obj, name.as_ptr())
}

macro_rules! impl_named_uniform_vec_n {
    ($name:ident [$t:ty;$n:expr] $f:ident) => {
        pub unsafe fn $name(&mut self, name: &str, v: [$t;$n]) {
            let loc = get_uniform_location(self.prog, name);
            self.gl.$f(loc, $n, v.as_ptr());
        }
    };
}

impl<'a> UniformCtx<'a> {
    pub unsafe fn set_named_uniform_float(&mut self, name: &str, v: f32) {
        let loc = get_uniform_location(self.prog, name);
        self.gl.Uniform1f(loc, v);
    }

    pub unsafe fn set_named_uniform_int(&mut self, name: &str, v: i32) {
        let loc = get_uniform_location(self.prog, name);
        self.gl.Uniform1i(loc, v);
    }

    impl_named_uniform_vec_n!(set_named_uniform_vec2 [f32;2] Uniform2fv);
    impl_named_uniform_vec_n!(set_named_uniform_vec3 [f32;3] Uniform3fv);
    impl_named_uniform_vec_n!(set_named_uniform_vec4 [f32;4] Uniform4fv);

    impl_named_uniform_vec_n!(set_named_uniform_ivec2 [i32;2] Uniform2iv);
    impl_named_uniform_vec_n!(set_named_uniform_ivec3 [i32;3] Uniform3iv);
    impl_named_uniform_vec_n!(set_named_uniform_ivec4 [i32;4] Uniform4iv);

    pub unsafe fn set_texture_2d(&mut self, tex_unit: u32, tex: &TextureHandle) {
        unimplemented!()
    }
}
