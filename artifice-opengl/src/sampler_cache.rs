use artifice_gfxbase::sampling::{SamplerDescription, Filter, SamplerMipmapMode, SamplerAddressMode};
use crate::api::gl::types::*;
use crate::api::{Gl, gl};
use std::collections::HashMap;

impl_handle_type!(pub struct SamplerHandle(GLuint));

pub struct SamplerCache {
    samplers: HashMap<SamplerDescription, SamplerHandle>,
}

impl SamplerCache {
    pub fn new() -> SamplerCache {
        unimplemented!()
        /*SamplerCache {
            samplers: HashMap::with_hasher(FxBuildHasher::default()),
        }*/
    }

    pub fn get_sampler(&mut self, gl: &Gl, desc: &SamplerDescription) -> &SamplerHandle {
        self.samplers.entry(desc.clone()).or_insert_with(|| unsafe {
            let mut obj = 0;
            gl.GenSamplers(1, &mut obj);
            gl.SamplerParameteri(
                obj,
                gl::TEXTURE_MIN_FILTER,
                min_filter_to_glenum(desc.min_filter, desc.mipmap_mode) as i32,
            );
            gl.SamplerParameteri(
                obj,
                gl::TEXTURE_MAG_FILTER,
                mag_filter_to_glenum(desc.mag_filter) as i32,
            );
            gl.SamplerParameteri(
                obj,
                gl::TEXTURE_WRAP_R,
                address_mode_to_glenum(desc.addr_u) as i32,
            );
            gl.SamplerParameteri(
                obj,
                gl::TEXTURE_WRAP_S,
                address_mode_to_glenum(desc.addr_v) as i32,
            );
            gl.SamplerParameteri(
                obj,
                gl::TEXTURE_WRAP_T,
                address_mode_to_glenum(desc.addr_w) as i32,
            );
            SamplerHandle::from_raw(gl, obj)
        })
    }
}

fn min_filter_to_glenum(filter: Filter, mipmap_mode: SamplerMipmapMode) -> GLenum {
    match (filter, mipmap_mode) {
        (Filter::Nearest, SamplerMipmapMode::Linear) => gl::NEAREST_MIPMAP_LINEAR,
        (Filter::Linear, SamplerMipmapMode::Linear) => gl::LINEAR_MIPMAP_LINEAR,
        (Filter::Nearest, SamplerMipmapMode::Nearest) => gl::NEAREST_MIPMAP_NEAREST,
        (Filter::Linear, SamplerMipmapMode::Nearest) => gl::LINEAR_MIPMAP_NEAREST,
    }
}

fn mag_filter_to_glenum(filter: Filter) -> GLenum {
    match filter {
        Filter::Nearest => gl::NEAREST,
        Filter::Linear => gl::LINEAR,
    }
}

fn address_mode_to_glenum(mode: SamplerAddressMode) -> GLenum {
    match mode {
        SamplerAddressMode::Clamp => gl::CLAMP_TO_EDGE,
        SamplerAddressMode::Mirror => gl::MIRRORED_REPEAT,
        SamplerAddressMode::Wrap => gl::REPEAT,
    }
}
