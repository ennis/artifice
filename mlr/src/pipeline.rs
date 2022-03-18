use crate::{shader::ShaderModule, vk::GraphicsPipelineCreateInfo, Arguments, VertexAttribute, VertexData};
use bitflags::bitflags;
use graal::vk;
use mlr::{device::Device, vertex::VertexInputInterface};
use std::{ffi::c_void, marker::PhantomData, mem, os::raw::c_char, ptr, sync::Arc};

//--------------------------------------------------------------------------------------------------

pub trait FormatT {
    const FORMAT: vk::Format;
}

macro_rules! impl_format_ty {
    ($t:ident : $format:ident) => {
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
        pub struct $t;
        impl FormatT for $t {
            const FORMAT: vk::Format = vk::Format::$format;
        }
    };
}

impl_format_ty!(RGBA16Float: R16G16B16A16_SFLOAT);
impl_format_ty!(RGBA8: R8G8B8A8_UNORM);
impl_format_ty!(RG16Float: R16G16_SFLOAT);

pub trait FragmentOutputColorInterface {
    const COLOR_ATTACHMENT_FORMATS: &'static [vk::Format];
    //const DEPTH_ATTACHMENT_FORMAT: Option<vk::Format>;
}

macro_rules! impl_tuple_fragment_output_interface {
    (
        @impl $($t:ident)*
    ) => {
        impl <$($t,)*> FragmentOutputColorInterface for ($($t,)*) where $($t: FormatT,)* {
            const COLOR_ATTACHMENT_FORMATS: &'static [vk::Format] = &[ $($t::FORMAT,)* ];
        }
    };

    (
        $t:ident
    ) => {
        impl_tuple_fragment_output_interface!(@impl $t);
    };

    (
        $t:ident $($ts:ident)+
    ) => {
        impl_tuple_fragment_output_interface!(@impl $t $($ts)*);
        impl_tuple_fragment_output_interface!($($ts)*);
    };
}

impl_tuple_fragment_output_interface!(T0 T1 T2 T3 T4 T5 T6 T7 T8 T9 T10 T11 T12);

//--------------------------------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub struct VertexBufferView<T: VertexData> {
    pub buffer: vk::Buffer,
    pub offset: vk::DeviceSize,
    pub _phantom: PhantomData<*const T>,
}

pub trait VertexBindingInterface {
    const ATTRIBUTES: &'static [VertexAttribute];
    const STRIDE: usize;
}

impl<T: VertexData> VertexBindingInterface for VertexBufferView<T> {
    const ATTRIBUTES: &'static [VertexAttribute] = T::ATTRIBUTES;
    const STRIDE: usize = mem::size_of::<T>();
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VertexInputBindingAttributes<'a> {
    pub base_location: u32,
    pub attributes: &'a [VertexAttribute],
}

pub trait VertexInputInterface {
    const BINDINGS: &'static [vk::VertexInputBindingDescription];
    const ATTRIBUTES: &'static [vk::VertexInputAttributeDescription];
}

macro_rules! impl_tuple_vertex_input_interface {
    (
        @impl $($t:ident)*
    ) => {
        impl <$($t,)*> VertexInputInterface for ($($t,)*) where $($t: VertexBindingInterface,)* {
            const BINDINGS: &'static [vk::VertexInputBindingDescription] = {
                let mut binding_counter = 0u32;
                &[
                     $(
                        {
                            let binding = binding_counter;
                            binding_counter += 1;
                             vk::VertexInputBindingDescription {
                                binding,
                                stride: $t::STRIDE,
                                input_rate: vk::VertexInputRate::VERTEX,
                             }
                        },
                     )*
                 ]
            };

            const ATTRIBUTES: &'static [vk::VertexInputAttributeDescription] = {
                let attrs = &[];
                let mut binding = 0;
                let mut base_location = 0;
                $(
                    let attrs = vertex_macro_helpers::append_attributes(attrs, binding, base_location, $t::ATTRIBUTES);
                    binding += 1;
                    base_location += $t::ATTRIBUTES.len();
                )*
                attrs
            };
        }
    };

    (
        $t:ident
    ) => {
        impl_tuple_vertex_input_interface!(@impl $t);
    };

    (
        $t:ident $($ts:ident)+
    ) => {
        impl_tuple_vertex_input_interface!(@impl $t $($ts)*);
        impl_tuple_vertex_input_interface!($($ts)*);
    };
}

impl_tuple_vertex_input_interface!(T0 T1 T2 T3);

/// Extension trait for VertexInputInterface
pub trait VertexInputInterfaceExt: VertexInputInterface {
    /// Helper function to get a `vk::PipelineVertexInputStateCreateInfo` from this vertex input struct.
    fn get_pipeline_vertex_input_state_create_info() -> vk::PipelineVertexInputStateCreateInfo;
}

/*impl<T: VertexInputInterface> VertexInputInterfaceExt for T {
    fn get_pipeline_vertex_input_state_create_info() -> vk::PipelineVertexInputStateCreateInfo {
        vk::PipelineVertexInputStateCreateInfo {
            vertex_binding_description_count: Self::BINDINGS.len() as u32,
            p_vertex_binding_descriptions: Self::BINDINGS.as_ptr(),
            vertex_attribute_description_count: Self::ATTRIBUTES.len() as u32,
            p_vertex_attribute_descriptions: Self::ATTRIBUTES.as_ptr(),
            ..Default::default()
        }
    }
}*/

pub mod vertex_macro_helpers {
    use graal::vk;
    use mlr::vertex::VertexAttribute;

    pub const fn append_attributes<const N: usize>(
        head: &'static [vk::VertexInputAttributeDescription],
        binding: u32,
        base_location: u32,
        tail: &'static [VertexAttribute],
    ) -> [vk::VertexInputAttributeDescription; N] {
        const NULL_ATTR: vk::VertexInputAttributeDescription = vk::VertexInputAttributeDescription {
            location: 0,
            binding: 0,
            format: vk::Format::UNDEFINED,
            offset: 0,
        };
        let mut result = [NULL_ATTR; N];
        let mut i = 0;
        while i < head.len() {
            result[i] = head[i];
            i += 1;
        }
        while i < N {
            let j = i - head.len();
            result[i] = vk::VertexInputAttributeDescription {
                location: base_location + j as u32,
                binding,
                format: tail[j].format,
                offset: tail[j].offset,
            };
            i += 1;
        }

        result
    }
}

//--------------------------------------------------------------------------------------------------

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
        //unsafe {}
        todo!()
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
    pub fn new_vertex_fragment(vertex: ShaderModule, fragment: ShaderModule) -> GraphicsShaderStages {
        GraphicsShaderStages {
            vertex,
            fragment: Some(fragment),
            geometry: None,
            tess_control: None,
            tess_eval: None,
        }
    }
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
    //pub strip_index_format: Option<IndexFormat>,
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

/// Create info for a graphics pipeline (excluding interfaces).
pub struct GraphicsPipelineConfig<'a> {
    pub vertex_shader: &'a ShaderModule,
    pub fragment_shader: &'a ShaderModule,
    pub primitive_state: PrimitiveState,
    pub multisample_state: MultisampleState,
    pub depth_stencil_state: Option<DepthStencilState>,
    pub color_attachments: &'a [ColorTargetState],
}

pub struct PipelineInterfaceDesc<'a> {
    vertex_bindings: &'a [vk::VertexInputBindingDescription],
    vertex_attributes: &'a [vk::VertexInputAttributeDescription],
    descriptor_set_layouts: &'a [vk::DescriptorSetLayout],
    color_attachment_formats: &'a [vk::Format],
    depth_attachment_format: Option<vk::Format>,
    stencil_attachment_format: Option<vk::Format>,
}

pub trait PipelineInterface {
    const VERTEX_INPUT_BINDINGS: &'static [vk::VertexInputBindingDescription];
    const VERTEX_INPUT_ATTRIBUTES: &'static [vk::VertexInputAttributeDescription];
    const COLOR_ATTACHMENT_FORMATS: &'static [vk::Format];
    const DEPTH_ATTACHMENT_FORMAT: Option<vk::Format>;
    const DESCRIPTOR_SET_LAYOUTS: &'static []
}

pub struct RawGraphicsPipeline {
    device: Arc<graal::Device>,
    pipeline: vk::Pipeline,
}

impl RawGraphicsPipeline {
    pub unsafe fn new(
        device: &Arc<graal::Device>,
        config: &GraphicsPipelineConfig,
        interface: &PipelineInterfaceDesc,
    ) -> RawGraphicsPipeline {
        let mut pipeline_shader_stages = Vec::new();
        pipeline_shader_stages.push(vk::PipelineShaderStageCreateInfo {
            flags: vk::PipelineShaderStageCreateFlags::empty(),
            stage: vk::ShaderStageFlags::VERTEX,
            module: config.vertex_shader.get_or_create_shader_module(device),
            p_name: b"main\0".as_ptr() as *const c_char,
            p_specialization_info: ::std::ptr::null(),
            ..Default::default()
        });
        pipeline_shader_stages.push(vk::PipelineShaderStageCreateInfo {
            flags: vk::PipelineShaderStageCreateFlags::empty(),
            stage: vk::ShaderStageFlags::FRAGMENT,
            module: config.fragment_shader.get_or_create_shader_module(device),
            p_name: b"main\0".as_ptr() as *const c_char,
            p_specialization_info: ::std::ptr::null(),
            ..Default::default()
        });

        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo {
            flags: vk::PipelineVertexInputStateCreateFlags::empty(),
            vertex_binding_description_count: interface.vertex_bindings.len() as u32,
            p_vertex_binding_descriptions: interface.vertex_bindings.as_ptr(),
            vertex_attribute_description_count: interface.vertex_attributes.len() as u32,
            p_vertex_attribute_descriptions: interface.vertex_attributes.as_ptr(),
            ..Default::default()
        };

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo {
            flags: vk::PipelineInputAssemblyStateCreateFlags::empty(),
            topology: config.primitive_state.topology.to_vk(),
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
            polygon_mode: config.primitive_state.polygon_mode.to_vk(),
            cull_mode: vk::CullModeFlags::NONE, // TODO
            front_face: config.primitive_state.front_face.to_vk(),
            depth_bias_enable: vk::FALSE, // TODO
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
            ..Default::default()
        };

        let multisample_state = vk::PipelineMultisampleStateCreateInfo {
            flags: Default::default(),
            rasterization_samples: match config.multisample_state.count {
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
            alpha_to_coverage_enable: if config.multisample_state.alpha_to_coverage_enabled {
                vk::TRUE
            } else {
                vk::FALSE
            },
            alpha_to_one_enable: vk::FALSE,
            ..Default::default()
        };

        let depth_stencil_state = if let Some(ref dss) = config.depth_stencil_state {
            vk::PipelineDepthStencilStateCreateInfo {
                flags: Default::default(),
                depth_test_enable: vk::TRUE,
                depth_write_enable: if dss.depth_write_enabled { vk::TRUE } else { vk::FALSE },
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

        let mut color_blend_attachments = Vec::with_capacity(config.color_attachments.len());
        for cts in config.color_attachments {
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

        // create pipeline layout
        let pipeline_layout = {
            let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo {
                flags: vk::PipelineLayoutCreateFlags::empty(),
                set_layout_count: interface.descriptor_set_layouts.len() as u32,
                p_set_layouts: interface.descriptor_set_layouts.as_ptr(),
                push_constant_range_count: 0,
                p_push_constant_ranges: ptr::null(),
                ..Default::default()
            };
            device
                .device
                .create_pipeline_layout(&pipeline_layout_create_info, None)
                .expect("failed to create pipeline layout")
        };

        // VK_KHR_dynamic_rendering
        let rendering_info = vk::PipelineRenderingCreateInfo {
            view_mask: 0,
            color_attachment_count: interface.color_attachment_formats.len() as u32,
            p_color_attachment_formats: interface.color_attachment_formats.as_ptr(),
            depth_attachment_format: interface.depth_attachment_format.unwrap_or(vk::Format::UNDEFINED),
            stencil_attachment_format: interface.stencil_attachment_format.unwrap_or(vk::Format::UNDEFINED),
            ..Default::default()
        };

        let create_info = vk::GraphicsPipelineCreateInfo {
            p_next: &rendering_info as *const _ as *const c_void,
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
            layout: pipeline_layout,
            render_pass: Default::default(),
            subpass: 0,
            base_pipeline_handle: Default::default(),
            base_pipeline_index: 0,
            ..Default::default()
        };

        unsafe {
            let pipelines = device
                .device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None)
                .expect("failed to create pipeline");
            RawGraphicsPipeline {
                device: device.clone(),
                pipeline: pipelines[0],
            }
        }
    }
}

pub struct GraphicsPipeline<VertexInput, FragmentOutputColor, ShaderResources> {
    raw: RawGraphicsPipeline,
    _vertex: PhantomData<VertexInput>,
    _fragment: PhantomData<FragmentOutputColor>,
    _shader: PhantomData<ShaderResources>,
}

pub struct GraphicsPipelineBuilder<VertexInput, FragmentOutputColor, ShaderResources> {
    _vertex: PhantomData<VertexInput>,
    _fragment: PhantomData<FragmentOutputColor>,
    _shader: PhantomData<ShaderResources>,
}

impl<VertexInput, FragmentOutputColor, ShaderResources>
    GraphicsPipelineBuilder<VertexInput, FragmentOutputColor, ShaderResources>
{
    /// Begin building a graphics pipeline.
    pub fn new() -> GraphicsPipelineBuilder<VertexInput, FragmentOutputColor, ShaderResources> {
        GraphicsPipelineBuilder {
            _vertex: PhantomData,
            _fragment: PhantomData,
            _shader: PhantomData,
        }
    }

    /// Specifies the type of the fragment output interface (color attachments).
    pub fn with_fragment_output<FO: FragmentOutputColorInterface>(
        self,
    ) -> GraphicsPipelineBuilder<VertexInput, FO, ShaderResources> {
        GraphicsPipelineBuilder {
            _vertex: PhantomData,
            _fragment: PhantomData,
            _shader: PhantomData,
        }
    }

    pub fn with_vertex_input<VI: VertexInputInterface>(
        self,
    ) -> GraphicsPipelineBuilder<VI, FragmentOutputColor, ShaderResources> {
        GraphicsPipelineBuilder {
            _vertex: PhantomData,
            _fragment: PhantomData,
            _shader: PhantomData,
        }
    }
}

impl<VertexInput, FragmentOutputColor, ShaderResources>
    GraphicsPipelineBuilder<VertexInput, FragmentOutputColor, ShaderResources>
where
    VertexInput: VertexInputInterface,
    FragmentOutputColor: FragmentOutputColorInterface,
    ShaderResources: ShaderResourceInterface,
{
    pub fn build(
        self,
        device: &Arc<graal::Device>,
        config: &GraphicsPipelineConfig,
    ) -> GraphicsPipeline<VertexInput, FragmentOutputColor, ShaderResources> {
        let interface_desc = PipelineInterfaceDesc {
            vertex_bindings: VertexInput::BINDINGS,
            vertex_attributes: VertexInput::ATTRIBUTES,
            descriptor_set_layouts: &[],
            color_attachment_formats: FragmentOutputColorInterface::COLOR_ATTACHMENT_FORMATS,
            depth_attachment_format: None,
            stencil_attachment_format: None,
        };
        unsafe {
            GraphicsPipeline {
                raw: RawGraphicsPipeline::new(device, config, &interface_desc),
                _vertex: PhantomData,
                _fragment: PhantomData,
                _shader: PhantomData,
            }
        }
    }
}

pub fn draw<VI, FO, SR>(
    frame: &mut graal::Frame<()>,
    pipeline: &mut GraphicsPipeline<VI, FO, SR>,
    vertex_input: &VI,
    shader_resources: &SR,
    fragment_output_color: &FO,
) where
    VI: VertexInputInterface,
    SR: ShaderResourceInterface,
    FO: FragmentOutputColorInterface,
{
}
