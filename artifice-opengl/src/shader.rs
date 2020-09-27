use std::ffi::{c_void, CString};
use std::mem;

use crate::api::Gl;
use crate::api::gl;
use crate::api::gl::types::*;
use crate::error::Error;

impl_handle_type!(pub struct ProgramHandle(GLuint));
impl_handle_type!(pub struct VertexShaderHandle(GLuint));
impl_handle_type!(pub struct FragmentShaderHandle(GLuint));

impl Drop for VertexShaderHandle {
    fn drop(&mut self) {
        unsafe { self.gl.DeleteShader(self.obj) }
    }
}

impl Drop for FragmentShaderHandle {
    fn drop(&mut self) {
        unsafe { self.gl.DeleteShader(self.obj) }
    }
}

impl Drop for ProgramHandle {
    fn drop(&mut self) {
        unsafe { self.gl.DeleteProgram(self.obj) }
    }
}

fn get_shader_info_log(gl: &Gl, obj: GLuint) -> String {
    unsafe {
        let mut log_size: GLint = 1000;
        let mut log_buf: Vec<u8> = Vec::with_capacity(log_size as usize);
        gl.GetShaderInfoLog(
            obj,
            log_size,
            &mut log_size,
            log_buf.as_mut_ptr() as *mut i8,
        );
        log_buf.set_len(log_size as usize);
        String::from_utf8(log_buf).unwrap()
    }
}

unsafe fn compile_glsl(gl: &Gl, source: &str, stage: GLenum) -> Result<GLuint, Error> {
    let obj = gl.CreateShader(stage);
    let srcs = [source.as_ptr() as *const i8];
    let lens = [source.len() as GLint];
    gl.ShaderSource(
        obj,
        1,
        &srcs[0] as *const *const i8,
        &lens[0] as *const GLint,
    );
    gl.CompileShader(obj);
    let mut status: GLint = 0;
    gl.GetShaderiv(obj, gl::COMPILE_STATUS, &mut status);
    if status != gl::TRUE as GLint {
        let log = get_shader_info_log(gl, obj);
        gl.DeleteShader(obj);
        Err(Error::ShaderCompilationError(log))
    } else {
        Ok(obj)
    }
}

unsafe fn create_from_spirv(gl: &Gl, stage: GLenum, bytecode: &[u32]) -> Result<GLuint, Error> {
    let mut obj = gl.CreateShader(stage);
    gl.ShaderBinary(
        1,
        &mut obj,
        gl::SHADER_BINARY_FORMAT_SPIR_V,
        bytecode.as_ptr() as *const c_void,
        mem::size_of_val(bytecode) as i32,
    );
    let entry_point = CString::new("main").unwrap();
    // TODO specialization constants
    gl.SpecializeShader(
        obj,
        entry_point.as_ptr(),
        0,
        0 as *const GLuint,
        0 as *const GLuint,
    );
    let mut status: GLint = 0;
    gl.GetShaderiv(obj, gl::COMPILE_STATUS, &mut status);
    if status != gl::TRUE as GLint {
        let log = get_shader_info_log(gl, obj);
        gl.DeleteShader(obj);
        Err(Error::ShaderCompilationError(log))
    } else {
        Ok(obj)
    }
}

macro_rules! impl_shader {
    ($handle:ty; $stage:expr) => {
        impl $handle {
            pub fn from_glsl(gl: &Gl, source: &str) -> Result<$handle, Error> {
                unsafe {
                    Ok(Self::from_raw(gl, compile_glsl(gl, source, $stage)?))
                }
            }
        }
    };
}

impl_shader!(VertexShaderHandle; gl::VERTEX_SHADER);
impl_shader!(FragmentShaderHandle; gl::FRAGMENT_SHADER);

impl ProgramHandle {
    pub fn link(
        gl: &Gl,
        vertex: &VertexShaderHandle,
        fragment: &FragmentShaderHandle,
    ) -> Result<ProgramHandle, Error> {
        unsafe {
            let obj = gl.CreateProgram();
            gl.AttachShader(obj, vertex.obj);
            gl.AttachShader(obj, fragment.obj);
            gl.LinkProgram(obj);
            let mut status: GLint = 0;
            let mut log_size: GLint = 0;
            gl.GetProgramiv(obj, gl::LINK_STATUS, &mut status);
            gl.GetProgramiv(obj, gl::INFO_LOG_LENGTH, &mut log_size);
            if status != gl::TRUE as GLint {
                let mut log_buf: Vec<u8> = Vec::with_capacity(log_size as usize);
                gl.GetProgramInfoLog(
                    obj,
                    log_size,
                    &mut log_size,
                    log_buf.as_mut_ptr() as *mut i8,
                );
                log_buf.set_len(log_size as usize);
                gl.DeleteProgram(obj);
                Err(Error::ProgramLinkError(String::from_utf8(log_buf).unwrap()))
            } else {
                Ok(ProgramHandle::from_raw(gl, obj))
            }
        }
    }
}
