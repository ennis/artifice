use crate::render::gl::handle::VertexArrayHandle;

/*#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct VertexAttribute {
    pub location: Option<u32>,
    pub ty: &'tcx TypeDesc,
    pub semantic: Option<Semantic<'tcx>>,
}*/

impl VertexArrayHandle {
    // don't know yet how and when we will create vertex arrays, so don't

    /* pub unsafe fn new(gl: &Gl, attribs: &[VertexInputAttributeDescription]) -> VertexArrayHandle {

        let mut vao = 0;
        gl.CreateVertexArrays(1, &mut vao);

        for a in attribs.iter() {
            gl.EnableVertexArrayAttrib(vao, a.location);
            let fmtinfo = a.format.get_format_info();
            let normalized = fmtinfo.is_normalized() as u8;
            let size = fmtinfo.num_components() as i32;
            let glfmt = GlFormatInfo::from_format(a.format);
            let ty = glfmt.upload_ty;

            gl.VertexArrayAttribFormat(vao, a.location, size, ty, normalized, a.offset);
            gl.VertexArrayAttribBinding(vao, a.location, a.binding);
        }

        vao
    }*/
}
