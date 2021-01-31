#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum SamplerAddressMode {
    Clamp,
    Mirror,
    Wrap,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum Filter {
    Nearest,
    Linear,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum SamplerMipmapMode {
    Nearest,
    Linear,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SamplerDescription {
    pub addr_u: SamplerAddressMode,
    pub addr_v: SamplerAddressMode,
    pub addr_w: SamplerAddressMode,
    pub min_filter: Filter,
    pub mag_filter: Filter,
    pub mipmap_mode: SamplerMipmapMode,
    pub border_color: [f32; 4],
}

impl SamplerDescription {
    pub const LINEAR_MIPMAP_LINEAR: SamplerDescription = SamplerDescription {
        addr_u: SamplerAddressMode::Clamp,
        addr_v: SamplerAddressMode::Clamp,
        addr_w: SamplerAddressMode::Clamp,
        mag_filter: Filter::Linear,
        min_filter: Filter::Linear,
        mipmap_mode: SamplerMipmapMode::Linear,
        border_color: [0.0, 0.0, 0.0, 0.0],
    };

    pub const LINEAR_MIPMAP_NEAREST: SamplerDescription = SamplerDescription {
        addr_u: SamplerAddressMode::Clamp,
        addr_v: SamplerAddressMode::Clamp,
        addr_w: SamplerAddressMode::Clamp,
        mag_filter: Filter::Linear,
        min_filter: Filter::Linear,
        mipmap_mode: SamplerMipmapMode::Nearest,
        border_color: [0.0, 0.0, 0.0, 0.0],
    };

    pub const NEAREST_MIPMAP_LINEAR: SamplerDescription = SamplerDescription {
        addr_u: SamplerAddressMode::Clamp,
        addr_v: SamplerAddressMode::Clamp,
        addr_w: SamplerAddressMode::Clamp,
        mag_filter: Filter::Nearest,
        min_filter: Filter::Nearest,
        mipmap_mode: SamplerMipmapMode::Linear,
        border_color: [0.0, 0.0, 0.0, 0.0],
    };

    pub const NEAREST_MIPMAP_NEAREST: SamplerDescription = SamplerDescription {
        addr_u: SamplerAddressMode::Clamp,
        addr_v: SamplerAddressMode::Clamp,
        addr_w: SamplerAddressMode::Clamp,
        mag_filter: Filter::Nearest,
        min_filter: Filter::Nearest,
        mipmap_mode: SamplerMipmapMode::Nearest,
        border_color: [0.0, 0.0, 0.0, 0.0],
    };

    pub const WRAP_NEAREST_MIPMAP_NEAREST: SamplerDescription = SamplerDescription {
        addr_u: SamplerAddressMode::Wrap,
        addr_v: SamplerAddressMode::Wrap,
        addr_w: SamplerAddressMode::Wrap,
        mag_filter: Filter::Nearest,
        min_filter: Filter::Nearest,
        mipmap_mode: SamplerMipmapMode::Nearest,
        border_color: [0.0, 0.0, 0.0, 0.0],
    };
}
