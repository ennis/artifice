pub mod vertex;
pub mod descriptor;
pub mod context;
pub mod image;
pub mod shader;
pub mod pipeline;
pub mod sampler;

// macro support
//extern crate self as mlr;

use std::cell::{Cell, RefCell};

struct ContextCache<T> {
    /// uniquely identifies the context that put the value there
    context_id: u64,
    value: RefCell<Option<T>>
}

struct ImageBackend {
    //
}

struct BufferBackend {

}

struct Buffer {

    // buffer on the GPU or CPU OR function result OR static data reference OR local owned data
    //
    // lifetime: current frame, etc.
    // lifetime: scope
    // by default: constructor

    cpu: Box<[u8]>, // just some data
    gpu_backend: ContextCache<BufferBackend>,
}

pub struct BufferView {

}

struct ShaderFunction {
    // GLSL source
    // unique ID/hash
    // uniforms
}

struct ColorTargets {
    // target images
}

