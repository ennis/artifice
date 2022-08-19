//! Sampler values.

use crate::model::value::ValueType;
use artifice::model::TypeDesc;
use std::hash::{Hash, Hasher};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum SamplerWrapMode {
    Clamp,
    Repeat,
    Mirror,
}

impl Default for SamplerWrapMode {
    fn default() -> Self {
        SamplerWrapMode::Clamp
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum SamplerFilter {
    Nearest,
    Linear,
}

impl Default for SamplerFilter {
    fn default() -> Self {
        SamplerFilter::Nearest
    }
}

/// Sampler parameters.
#[derive(Copy, Clone, Debug)]
pub struct SamplerParameters {
    pub wrap_mode_s: SamplerWrapMode,
    pub wrap_mode_t: SamplerWrapMode,
    pub wrap_mode_r: SamplerWrapMode,
    pub min_filter: SamplerFilter,
    pub mag_filter: SamplerFilter,
    pub border_color: glam::Vec4,
}

// required because we also have a custom hash impl
// (https://rust-lang.github.io/rust-clippy/master/index.html#derive_hash_xor_eq)
impl PartialEq for SamplerParameters {
    fn eq(&self, other: &Self) -> bool {
        self.wrap_mode_s == other.wrap_mode_s
            && self.wrap_mode_t == other.wrap_mode_t
            && self.wrap_mode_r == other.wrap_mode_r
            && self.min_filter == other.min_filter
            && self.mag_filter == other.mag_filter
            && self.border_color.x.to_bits() == other.border_color.x.to_bits()
            && self.border_color.y.to_bits() == other.border_color.y.to_bits()
            && self.border_color.z.to_bits() == other.border_color.z.to_bits()
            && self.border_color.w.to_bits() == other.border_color.w.to_bits()
    }
}

impl Hash for SamplerParameters {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.wrap_mode_s.hash(state);
        self.wrap_mode_t.hash(state);
        self.wrap_mode_r.hash(state);
        self.min_filter.hash(state);
        self.mag_filter.hash(state);
        self.border_color.x.to_bits().hash(state);
        self.border_color.y.to_bits().hash(state);
        self.border_color.z.to_bits().hash(state);
        self.border_color.w.to_bits().hash(state);
    }
}

impl Default for SamplerParameters {
    fn default() -> Self {
        SamplerParameters {
            wrap_mode_s: Default::default(),
            wrap_mode_t: Default::default(),
            wrap_mode_r: Default::default(),
            min_filter: Default::default(),
            mag_filter: Default::default(),
            border_color: Default::default(),
        }
    }
}

impl ValueType for SamplerParameters {
    fn hash(&self, mut hasher: &mut dyn Hasher) {
        Hash::hash(self, &mut hasher)
    }

    fn type_desc(&self) -> Option<&TypeDesc> {
        Some(&TypeDesc::Sampler)
    }
}
