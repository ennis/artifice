use crate::{shader::ShaderModule, vk::GraphicsPipelineCreateInfo};
use bitflags::bitflags;
use graal::vk;
use mlr::{Arguments, Device};
use std::{mem, os::raw::c_char, ptr, sync::Arc};

#[repr(transparent)]
pub struct ArgumentLayout {
    // FIXME these are never deleted, for now
    layout: vk::DescriptorSetLayout,
}

pub struct PipelineLayoutDescriptor<'a> {
    layouts: &'a [ArgumentLayout],
}

pub struct PipelineLayout {
    // FIXME these are never deleted, for now
    layout: vk::PipelineLayout,
}

impl PipelineLayout {
    pub fn new(device: &graal::Device, descriptor: &PipelineLayoutDescriptor) -> PipelineLayout {
        unsafe {
            let create_info = vk::PipelineLayoutCreateInfo {
                flags: vk::PipelineLayoutCreateFlags::empty(),
                set_layout_count: descriptor.layouts.len() as u32,
                p_set_layouts: descriptor.layouts.as_ptr() as *const vk::DescriptorSetLayout,
                push_constant_range_count: 0,
                p_push_constant_ranges: ptr::null(),
                ..Default::default()
            };
            let layout = device
                .device
                .create_pipeline_layout(&create_info, None)
                .expect("failed to create pipeline layout");
            PipelineLayout { layout }
        }
    }
}

pub struct GraphicsShaderStages {
    //pub format: ShaderFormat,
    pub vertex: ShaderModule,
    pub geometry: Option<ShaderModule>,
    pub fragment: Option<ShaderModule>,
    pub tess_eval: Option<ShaderModule>,
    pub tess_control: Option<ShaderModule>,
}

impl GraphicsShaderStages {
    pub fn new_vertex_fragment(
        vertex: ShaderModule,
        fragment: ShaderModule,
    ) -> GraphicsShaderStages {
        GraphicsShaderStages {
            vertex,
            fragment: Some(fragment),
            geometry: None,
            tess_control: None,
            tess_eval: None,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct VertexInputState<'a> {
    bindings: &'a [vk::VertexInputBindingDescription],
    attributes: &'a [vk::VertexInputAttributeDescription],
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CompareFunction {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

impl CompareFunction {
    /// Returns true if the comparison depends on the reference value.
    pub fn needs_ref_value(self) -> bool {
        match self {
            Self::Never | Self::Always => false,
            _ => true,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct StencilFaceState {
    pub compare: CompareFunction,
    pub fail_op: StencilOperation,
    pub depth_fail_op: StencilOperation,
    pub pass_op: StencilOperation,
}

impl StencilFaceState {
    /// Ignore the stencil state for the face.
    pub const IGNORE: Self = StencilFaceState {
        compare: CompareFunction::Always,
        fail_op: StencilOperation::Keep,
        depth_fail_op: StencilOperation::Keep,
        pass_op: StencilOperation::Keep,
    };

    /// Returns true if the face state uses the reference value for testing or operation.
    pub fn needs_ref_value(&self) -> bool {
        self.compare.needs_ref_value()
            || self.fail_op == StencilOperation::Replace
            || self.depth_fail_op == StencilOperation::Replace
            || self.pass_op == StencilOperation::Replace
    }
}

impl Default for StencilFaceState {
    fn default() -> Self {
        Self::IGNORE
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum StencilOperation {
    Keep,
    Zero,
    Replace,
    Invert,
    IncrementClamp,
    DecrementClamp,
    IncrementWrap,
    DecrementWrap,
}

#[derive(Copy, Clone, Debug)]
pub struct StencilState {
    pub front: StencilFaceState,
    pub back: StencilFaceState,
    pub read_mask: u32,
    pub write_mask: u32,
}

impl StencilState {
    /// Returns true if the stencil test is enabled.
    pub fn is_enabled(&self) -> bool {
        (self.front != StencilFaceState::IGNORE || self.back != StencilFaceState::IGNORE)
            && (self.read_mask != 0 || self.write_mask != 0)
    }
    /// Returns true if the state doesn't mutate the target values.
    pub fn is_read_only(&self) -> bool {
        self.write_mask == 0
    }
    /// Returns true if the stencil state uses the reference value for testing.
    pub fn needs_ref_value(&self) -> bool {
        todo!()
        //self.front.needs_ref_value() || self.back.needs_ref_value()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct DepthStencilState {
    pub format: vk::Format,
    pub depth_write_enabled: bool,
    pub depth_compare: vk::CompareOp,
    pub stencil: StencilState,
    pub bias: DepthBiasState,
}

#[derive(Copy, Clone, Debug)]
pub struct DepthBiasState {
    pub constant: i32,
    pub slope_scale: f32,
    pub clamp: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct BlendComponent {
    pub src_factor: vk::BlendFactor,
    pub dst_factor: vk::BlendFactor,
    pub operation: vk::BlendOp,
}

#[derive(Copy, Clone, Debug)]
pub struct BlendState {
    pub color: BlendComponent,
    pub alpha: BlendComponent,
}

bitflags::bitflags! {
    /// Color write mask. Disabled color channels will not be written to.
    #[repr(transparent)]
    pub struct ColorWrites: u32 {
        /// Enable red channel writes
        const RED = 1 << 0;
        /// Enable green channel writes
        const GREEN = 1 << 1;
        /// Enable blue channel writes
        const BLUE = 1 << 2;
        /// Enable alpha channel writes
        const ALPHA = 1 << 3;
        /// Enable red, green, and blue channel writes
        const COLOR = Self::RED.bits | Self::GREEN.bits | Self::BLUE.bits;
        /// Enable writes to all channels.
        const ALL = Self::RED.bits | Self::GREEN.bits | Self::BLUE.bits | Self::ALPHA.bits;
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ColorTargetState {
    pub format: vk::Format,
    pub blend: Option<BlendState>,
    pub write_mask: ColorWrites,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum IndexFormat {
    Uint16,
    Uint32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PolygonMode {
    Line,
    Fill,
    Point,
}

impl PolygonMode {
    fn to_vk(&self) -> vk::PolygonMode {
        match self {
            PolygonMode::Line => vk::PolygonMode::LINE,
            PolygonMode::Fill => vk::PolygonMode::FILL,
            PolygonMode::Point => vk::PolygonMode::POINT,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum FrontFace {
    CounterClockwise,
    Clockwise,
}

impl FrontFace {
    fn to_vk(&self) -> vk::FrontFace {
        match self {
            FrontFace::CounterClockwise => vk::FrontFace::COUNTER_CLOCKWISE,
            FrontFace::Clockwise => vk::FrontFace::CLOCKWISE,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Face {
    Front,
    Back,
}

impl Face {
    /*fn to_vk(&self) -> vk::FrontFace {
        match self {
            Face::Front => vk::FrontFace::COUNTER_CLOCKWISE,
            Face::Back => vk::FrontFace::CLOCKWISE,
        }
    }*/
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PrimitiveTopology {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip,
}

impl PrimitiveTopology {
    fn to_vk(&self) -> vk::PrimitiveTopology {
        match self {
            PrimitiveTopology::PointList => vk::PrimitiveTopology::POINT_LIST,
            PrimitiveTopology::LineList => vk::PrimitiveTopology::LINE_LIST,
            PrimitiveTopology::LineStrip => vk::PrimitiveTopology::LINE_STRIP,
            PrimitiveTopology::TriangleList => vk::PrimitiveTopology::TRIANGLE_LIST,
            PrimitiveTopology::TriangleStrip => vk::PrimitiveTopology::TRIANGLE_STRIP,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PrimitiveState {
    pub topology: PrimitiveTopology,
    pub strip_index_format: Option<IndexFormat>,
    pub front_face: FrontFace,
    pub cull_mode: Option<Face>,
    pub clamp_depth: bool,
    pub polygon_mode: PolygonMode,
    pub conservative: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct MultisampleState {
    pub count: u32,
    pub mask: u64,
    pub alpha_to_coverage_enabled: bool,
}

pub struct GraphicsPipelineDescriptor<'a> {
    vertex_input: VertexInputState<'a>,
    vertex_shader: &'a ShaderModule,
    fragment_shader: &'a ShaderModule,
    primitive_state: PrimitiveState,
    multisample_state: MultisampleState,
    depth_stencil_state: Option<DepthStencilState>,
    color_attachments: &'a [ColorTargetState],
}

pub struct GraphicsPipeline {
    device: Arc<graal::Device>,
    pipeline: vk::Pipeline,
}

impl Device {
    pub fn create_graphics_pipeline(
        &mut self,
        desc: &GraphicsPipelineDescriptor,
    ) -> GraphicsPipeline {
        let mut pipeline_shader_stages = Vec::new();
        pipeline_shader_stages.push(vk::PipelineShaderStageCreateInfo {
            flags: vk::PipelineShaderStageCreateFlags::empty(),
            stage: vk::ShaderStageFlags::VERTEX,
            module: desc.vertex_shader.shader_module,
            p_name: b"main\0".as_ptr() as *const c_char,
            p_specialization_info: ::std::ptr::null(),
            ..Default::default()
        });
        pipeline_shader_stages.push(vk::PipelineShaderStageCreateInfo {
            flags: vk::PipelineShaderStageCreateFlags::empty(),
            stage: vk::ShaderStageFlags::FRAGMENT,
            module: desc.fragment_shader.shader_module,
            p_name: b"main\0".as_ptr() as *const c_char,
            p_specialization_info: ::std::ptr::null(),
            ..Default::default()
        });

        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo {
            flags: vk::PipelineVertexInputStateCreateFlags::empty(),
            vertex_binding_description_count: desc.vertex_input.bindings.len() as u32,
            p_vertex_binding_descriptions: desc.vertex_input.bindings.as_ptr(),
            vertex_attribute_description_count: desc.vertex_input.attributes.len() as u32,
            p_vertex_attribute_descriptions: desc.vertex_input.attributes.as_ptr(),
            ..Default::default()
        };

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo {
            flags: vk::PipelineInputAssemblyStateCreateFlags::empty(),
            topology: desc.primitive_state.topology.to_vk(),
            primitive_restart_enable: vk::FALSE,
            ..Default::default()
        };

        let tessellation_state = vk::PipelineTessellationStateCreateInfo::default();

        let viewport_state = vk::PipelineViewportStateCreateInfo {
            flags: vk::PipelineViewportStateCreateFlags::empty(),
            viewport_count: 1,
            p_viewports: ptr::null(),
            scissor_count: 1,
            p_scissors: ptr::null(),
            ..Default::default()
        };

        let rasterization_state = vk::PipelineRasterizationStateCreateInfo {
            flags: Default::default(),
            depth_clamp_enable: vk::FALSE,
            rasterizer_discard_enable: vk::FALSE,
            polygon_mode: desc.primitive_state.polygon_mode.to_vk(),
            cull_mode: vk::CullModeFlags::NONE, // TODO
            front_face: desc.primitive_state.front_face.to_vk(),
            depth_bias_enable: vk::FALSE, // TODO
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
            ..Default::default()
        };

        let multisample_state = vk::PipelineMultisampleStateCreateInfo {
            flags: Default::default(),
            rasterization_samples: match desc.multisample_state.count {
                1 => vk::SampleCountFlags::TYPE_1,
                2 => vk::SampleCountFlags::TYPE_2,
                4 => vk::SampleCountFlags::TYPE_4,
                8 => vk::SampleCountFlags::TYPE_8,
                16 => vk::SampleCountFlags::TYPE_16,
                32 => vk::SampleCountFlags::TYPE_32,
                64 => vk::SampleCountFlags::TYPE_64,
                _ => panic!("unsupported sample count"),
            },
            sample_shading_enable: vk::FALSE,
            min_sample_shading: 0.0,
            p_sample_mask: ptr::null(),
            alpha_to_coverage_enable: if desc.multisample_state.alpha_to_coverage_enabled {
                vk::TRUE
            } else {
                vk::FALSE
            },
            alpha_to_one_enable: vk::FALSE,
            ..Default::default()
        };

        let depth_stencil_state = if let Some(ref dss) = desc.depth_stencil_state {
            vk::PipelineDepthStencilStateCreateInfo {
                flags: Default::default(),
                depth_test_enable: vk::TRUE,
                depth_write_enable: if dss.depth_write_enabled {
                    vk::TRUE
                } else {
                    vk::FALSE
                },
                depth_compare_op: dss.depth_compare,
                depth_bounds_test_enable: 0,
                stencil_test_enable: vk::FALSE, // TODO
                front: Default::default(),
                back: Default::default(),
                min_depth_bounds: 0.0,
                max_depth_bounds: 0.0,
                ..Default::default()
            }
        } else {
            vk::PipelineDepthStencilStateCreateInfo {
                flags: Default::default(),
                depth_test_enable: vk::FALSE,
                depth_write_enable: vk::FALSE,
                depth_compare_op: Default::default(),
                depth_bounds_test_enable: 0,
                stencil_test_enable: 0,
                front: Default::default(),
                back: Default::default(),
                min_depth_bounds: 0.0,
                max_depth_bounds: 0.0,
                ..Default::default()
            }
        };

        let mut color_blend_attachments = Vec::with_capacity(desc.color_attachments.len());
        for cts in desc.color_attachments {
            let color_blend_attachment = if let Some(blend) = cts.blend {
                vk::PipelineColorBlendAttachmentState {
                    blend_enable: vk::TRUE,
                    src_color_blend_factor: blend.color.src_factor,
                    dst_color_blend_factor: blend.color.dst_factor,
                    color_blend_op: blend.color.operation,
                    src_alpha_blend_factor: blend.alpha.src_factor,
                    dst_alpha_blend_factor: blend.alpha.dst_factor,
                    alpha_blend_op: blend.alpha.operation,
                    color_write_mask: vk::ColorComponentFlags::from_raw(cts.write_mask.bits),
                }
            } else {
                vk::PipelineColorBlendAttachmentState {
                    blend_enable: vk::FALSE,
                    src_color_blend_factor: Default::default(),
                    dst_color_blend_factor: Default::default(),
                    color_blend_op: Default::default(),
                    src_alpha_blend_factor: Default::default(),
                    dst_alpha_blend_factor: Default::default(),
                    alpha_blend_op: Default::default(),
                    color_write_mask: vk::ColorComponentFlags::from_raw(cts.write_mask.bits),
                }
            };
            color_blend_attachments.push(color_blend_attachment);
        }

        let color_blend_state = vk::PipelineColorBlendStateCreateInfo {
            flags: Default::default(),
            logic_op_enable: vk::FALSE, // TODO
            logic_op: Default::default(),
            attachment_count: color_blend_attachments.len() as u32,
            p_attachments: color_blend_attachments.as_ptr(),
            blend_constants: [0.0f32; 4],
            ..Default::default()
        };

        let dynamic_states = &[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];

        let dynamic_state = vk::PipelineDynamicStateCreateInfo {
            flags: Default::default(),
            dynamic_state_count: dynamic_states.len() as u32,
            p_dynamic_states: dynamic_states.as_ptr(),
            ..Default::default()
        };

        let create_info = vk::GraphicsPipelineCreateInfo {
            flags: vk::PipelineCreateFlags::empty(),
            stage_count: pipeline_shader_stages.len() as u32,
            p_stages: pipeline_shader_stages.as_ptr(),
            p_vertex_input_state: &vertex_input_state,
            p_input_assembly_state: &input_assembly_state,
            p_tessellation_state: &tessellation_state,
            p_viewport_state: &viewport_state,
            p_rasterization_state: &rasterization_state,
            p_multisample_state: &multisample_state,
            p_depth_stencil_state: &depth_stencil_state,
            p_color_blend_state: &color_blend_state,
            p_dynamic_state: &dynamic_state,
            layout: Default::default(),
            render_pass: Default::default(),
            subpass: 0,
            base_pipeline_handle: Default::default(),
            base_pipeline_index: 0,
            ..Default::default()
        };

        unsafe {
            let device = self.vulkan_device();
            let pipelines = device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None)
                .expect("failed to create pipeline");
            GraphicsPipeline {
                device: self.backend.clone(),
                pipeline: pipelines[0],
            }
        }
    }
}
