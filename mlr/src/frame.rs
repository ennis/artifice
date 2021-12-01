use crate::{
    buffer::BufferAny,
    context::Context,
    image::ImageAny,
    shader::{ArgumentBlock, ShaderArguments},
};
use graal::{
    ash::vk::{DescriptorType, ShaderStageFlags},
    vk, Device, FrameCreateInfo, ImageId, ResourceGroupId, ResourceId,
};
use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    io::Write,
    sync::Arc,
};

