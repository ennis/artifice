//! Sampler values.

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum SamplerWrapMode {
    Clamp,
    Repeat,
    Mirror,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum SamplerFilter {
    Nearest,
    Linear,
}

#[derive(Copy, Clone, Debug, PartialEq, Hash)]
pub struct Sampler {
    pub wrap_mode_s: SamplerWrapMode,
    pub wrap_mode_t: SamplerWrapMode,
    pub wrap_mode_r: SamplerWrapMode,
    pub min_filter: SamplerFilter,
    pub mag_filter: SamplerFilter,
    pub border_color: glam::Vec4,
}
