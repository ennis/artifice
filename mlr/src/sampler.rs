//! Sampler objects
use std::any::{Any, TypeId};
use std::sync::Arc;
use crate::vk::SamplerCreateInfo;
use graal::vk;
use crate::Device;
use crate::device::{Device, SamplerId};

// TODO should be called just "sampler"
pub trait SamplerType: Copy {
    fn unique_type_id(&self) -> Option<TypeId>;
    fn to_sampler(&self, device: &graal::ash::Device) -> vk::Sampler;
}

macro_rules! impl_static_sampler_type {
    ($v:vis $name:ident, $mag:ident, $min:ident, $mipmap_mode:ident, $addr_u:ident, $addr_v:ident, $addr_w:ident) => {
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
        $v struct $name;
        impl SamplerType for $name {
            fn unique_type_id(&self) -> Option<TypeId> {
                Some(::std::any::TypeId::of::<Self>())
            }

            fn to_sampler(&self, device: &graal::ash::Device) -> vk::Sampler {
                const SAMPLER_CREATE_INFO: vk::SamplerCreateInfo = vk::SamplerCreateInfo {
                    s_type: vk::StructureType::SAMPLER_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::SamplerCreateFlags::empty(),
                    mag_filter: vk::Filter::$mag,
                    min_filter: vk::Filter::$min,
                    mipmap_mode: vk::SamplerMipmapMode::$mipmap_mode,
                    address_mode_u: vk::SamplerAddressMode::$addr_u,
                    address_mode_v: vk::SamplerAddressMode::$addr_v,
                    address_mode_w: vk::SamplerAddressMode::$addr_w,
                    mip_lod_bias: 0.0,
                    anisotropy_enable: 0,
                    max_anisotropy: 0.0,
                    compare_enable: vk::FALSE,
                    compare_op: vk::CompareOp::ALWAYS,
                    min_lod: 0.0,
                    max_lod: 0.0,
                    border_color: vk::BorderColor::INT_OPAQUE_BLACK,
                    unnormalized_coordinates: 0,
                };
                unsafe {
                    // bail out if we can't create a simple sampler object with no particular extensions
                    device.create_sampler(&SAMPLER_CREATE_INFO, None).expect("failed to create static sampler")
                }
            }
        }
    };
}

impl_static_sampler_type!(pub Linear_ClampToEdge, LINEAR, LINEAR, LINEAR, CLAMP_TO_EDGE, CLAMP_TO_EDGE, CLAMP_TO_EDGE);
impl_static_sampler_type!(pub Nearest_ClampToEdge, NEAREST, NEAREST, NEAREST, CLAMP_TO_EDGE, CLAMP_TO_EDGE, CLAMP_TO_EDGE);

pub(crate) struct SamplerInner {
    pub(crate) device: Device,
    pub(crate) id: SamplerId,
    pub(crate) sampler: vk::Sampler,
}

#[derive(Clone)]
pub struct Sampler(pub(crate) Arc<SamplerInner>);

