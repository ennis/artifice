use crate::api::gl::types::*;
use crate::api::Gl;
use std::os::raw::c_void;
use std::ptr;

impl_handle_type!(pub struct BufferHandle(GLuint));

impl Drop for BufferHandle {
    fn drop(&mut self) {
        unsafe { self.gl.DeleteBuffers(1, &mut self.obj) }
    }
}

impl BufferHandle {
    pub unsafe fn new(gl: &Gl, size: usize, flags: GLenum) -> BufferHandle {
        let mut buffer = 0;
        gl.CreateBuffers(1, &mut buffer);
        gl.NamedBufferStorage(buffer, size as GLsizeiptr, ptr::null(), flags);
        BufferHandle::from_raw(gl, buffer)
    }

    pub unsafe fn with_data(
        gl: &Gl,
        size: usize,
        flags: GLenum,
        initial_data: *const c_void,
    ) -> BufferHandle {
        let mut buffer = 0;
        gl.CreateBuffers(1, &mut buffer);
        gl.NamedBufferStorage(buffer, size as GLsizeiptr, initial_data, flags);
        BufferHandle::from_raw(gl, buffer)
    }
}
