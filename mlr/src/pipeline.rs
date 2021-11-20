use graal::vk;

#[derive(Copy, Clone, Debug)]
pub struct GraphicsShaderStages<'a, 're, B: Backend> {
    //pub format: ShaderFormat,
    pub vertex: ShaderModule<'a, 're, B>,
    pub geometry: Option<ShaderModule<'a, 're, B>>,
    pub fragment: Option<ShaderModule<'a, 're, B>>,
    pub tess_eval: Option<ShaderModule<'a, 're, B>>,
    pub tess_control: Option<ShaderModule<'a, 're, B>>,
}

impl<'a, 're, B: Backend> GraphicsShaderStages<'a, 're, B> {
    pub fn new_vertex_fragment(
        vertex: ShaderModule<'a, 're, B>,
        fragment: ShaderModule<'a, 're, B>,
    ) -> GraphicsShaderStages<'a, 're, B> {
        GraphicsShaderStages {
            vertex,
            fragment: fragment.into(),
            geometry: None,
            tess_control: None,
            tess_eval: None,
        }
    }
}

bitflags! {
    #[derive(Default)]
    pub struct CullModeFlags: u32 {
        const NONE = 0;
        const FRONT = 1;
        const BACK = 2;
        const FRONT_AND_BACK = Self::FRONT.bits | Self::BACK.bits;
    }
}

bitflags! {
    #[derive(Default)]
    pub struct DynamicStateFlags: u32 {
        const VIEWPORT = (1 << 0);
        const SCISSOR = (1 << 1);
        const LINE_WIDTH = (1 << 2);
        const DEPTH_BIAS = (1 << 3);
        const BLEND_CONSTANTS = (1 << 4);
        const DEPTH_BOUNDS = (1 << 5);
        const STENCIL_COMPARE_MASK = (1 << 6);
        const STENCIL_WRITE_MASK = (1 << 7);
        const STENCIL_REFERENCE = (1 << 8);
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PolygonMode {
    Line,
    Fill,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum FrontFace {
    Clockwise,
    CounterClockwise,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum DepthBias {
    Disabled,
    Enabled {
        constant_factor: NotNan<f32>,
        clamp: NotNan<f32>,
        slope_factor: NotNan<f32>,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct RasterisationState {
    pub depth_clamp_enable: bool,
    pub rasterizer_discard_enable: bool,
    pub polygon_mode: PolygonMode,
    pub cull_mode: CullModeFlags,
    pub depth_bias: DepthBias,
    pub front_face: FrontFace,
    pub line_width: NotNan<f32>,
}

impl RasterisationState {
    pub const DEFAULT: RasterisationState = RasterisationState {
        depth_clamp_enable: false,
        rasterizer_discard_enable: false,
        polygon_mode: PolygonMode::Fill,
        cull_mode: CullModeFlags::NONE,
        depth_bias: DepthBias::Disabled,
        front_face: FrontFace::Clockwise,
        line_width: unsafe { mem::transmute(1.0f32) },
    };
}

impl Default for RasterisationState {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[repr(C)]
pub struct Viewport {
    pub x: NotNan<f32>,
    pub y: NotNan<f32>,
    pub width: NotNan<f32>,
    pub height: NotNan<f32>,
    pub min_depth: NotNan<f32>,
    pub max_depth: NotNan<f32>,
}

impl From<(u32, u32)> for Viewport {
    fn from((w, h): (u32, u32)) -> Self {
        Viewport {
            x: 0.0.into(),
            y: 0.0.into(),
            width: (w as f32).into(),
            height: (h as f32).into(),
            min_depth: 0.0.into(),
            max_depth: 1.0.into(),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[repr(C)]
pub struct ScissorRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Scissor {
    Enabled(ScissorRect),
    Disabled,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Viewports<'a> {
    Static(&'a [Viewport]),
    Dynamic,
}

impl<'a> From<Viewports<'a>> for ViewportsOwned {
    fn from(v: Viewports) -> Self {
        match v {
            Viewports::Static(v) => ViewportsOwned::Static(v.to_vec()),
            Viewports::Dynamic => ViewportsOwned::Dynamic,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum ViewportsOwned {
    Static(Vec<Viewport>),
    Dynamic,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Scissors<'a> {
    Static(&'a [Scissor]),
    Dynamic,
}

impl<'a> From<Scissors<'a>> for ScissorsOwned {
    fn from(s: Scissors) -> Self {
        match s {
            Scissors::Static(s) => ScissorsOwned::Static(s.to_vec()),
            Scissors::Dynamic => ScissorsOwned::Dynamic,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum ScissorsOwned {
    Static(Vec<Scissor>),
    Dynamic,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ViewportState<'a> {
    pub viewports: Viewports<'a>,
    pub scissors: Scissors<'a>,
}

impl<'a> Default for ViewportState<'a> {
    fn default() -> Self {
        ViewportState {
            scissors: Scissors::Static(&[Scissor::Disabled]),
            viewports: Viewports::Dynamic,
        }
    }
}

impl<'a> ViewportState<'a> {
    pub const DYNAMIC_VIEWPORT_SCISSOR: ViewportState<'static> = ViewportState {
        viewports: Viewports::Dynamic,
        scissors: Scissors::Dynamic,
    };
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct InputAssemblyState {
    pub topology: PrimitiveTopology,
    pub primitive_restart_enable: bool,
}

impl Default for InputAssemblyState {
    fn default() -> Self {
        InputAssemblyState {
            topology: PrimitiveTopology::TriangleList,
            primitive_restart_enable: false,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum SampleShading {
    Disabled,
    Enabled { min_sample_shading: NotNan<f32> },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct MultisampleState {
    pub rasterization_samples: u32,
    pub sample_shading: SampleShading,
    pub alpha_to_coverage_enable: bool,
    pub alpha_to_one_enable: bool,
}

impl Default for MultisampleState {
    fn default() -> Self {
        MultisampleState {
            rasterization_samples: 1,
            sample_shading: SampleShading::Disabled,
            alpha_to_coverage_enable: false,
            alpha_to_one_enable: false,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct AttachmentDescription {
    pub format: Format,
    pub samples: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct AttachmentLayout<'a> {
    pub input_attachments: &'a [AttachmentDescription],
    pub depth_attachment: Option<AttachmentDescription>,
    pub color_attachments: &'a [AttachmentDescription],
    //pub resolve_attachments: &'a [AttachmentDescription]
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum CompareOp {
    Never = 0,
    Less = 1,
    Equal = 2,
    LessOrEqual = 3,
    Greater = 4,
    NotEqual = 5,
    GreaterOrEqual = 6,
    Always = 7,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum StencilOp {
    Keep = 0,
    Zero = 1,
    Replace = 2,
    IncrementAndClamp = 3,
    DecrementAndClamp = 4,
    Invert = 5,
    IncrementAndWrap = 6,
    DecrementAndWrap = 7,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct StencilOpState {
    pub fail_op: StencilOp,
    pub pass_op: StencilOp,
    pub depth_fail_op: StencilOp,
    pub compare_op: CompareOp,
    pub compare_mask: u32,
    pub write_mask: u32,
    pub reference: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum DepthBoundTest {
    Disabled,
    Enabled {
        min_depth_bounds: NotNan<f32>,
        max_depth_bounds: NotNan<f32>,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum StencilTest {
    Disabled,
    Enabled {
        front: StencilOpState,
        back: StencilOpState,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct DepthStencilState {
    pub depth_test_enable: bool,
    pub depth_write_enable: bool,
    pub depth_compare_op: CompareOp,
    pub depth_bounds_test: DepthBoundTest,
    pub stencil_test: StencilTest,
}

impl Default for DepthStencilState {
    fn default() -> Self {
        DepthStencilState {
            depth_test_enable: false,
            depth_write_enable: false,
            depth_compare_op: CompareOp::Less,
            depth_bounds_test: DepthBoundTest::Disabled,
            stencil_test: StencilTest::Disabled,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum LogicOp {
    Clear = 0,
    And = 1,
    AndReverse = 2,
    Copy = 3,
    AndInverted = 4,
    NoOp = 5,
    Xor = 6,
    Or = 7,
    Nor = 8,
    Equivalent = 9,
    Invert = 10,
    OrReverse = 11,
    CopyInverted = 12,
    OrInverted = 13,
    Nand = 14,
    Set = 15,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum BlendFactor {
    Zero = 0,
    One = 1,
    SrcColor = 2,
    OneMinusSrcColor = 3,
    DstColor = 4,
    OneMinusDstColor = 5,
    SrcAlpha = 6,
    OneMinusSrcAlpha = 7,
    DstAlpha = 8,
    OneMinusDstAlpha = 9,
    ConstantColor = 10,
    OneMinusConstantColor = 11,
    ConstantAlpha = 12,
    OneMinusConstantAlpha = 13,
    SrcAlphaSaturate = 14,
    Src1Color = 15,
    OneMinusSrc1Color = 16,
    Src1Alpha = 17,
    OneMinusSrc1Alpha = 18,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum BlendOp {
    Add = 0,
    Subtract = 1,
    ReverseSubtract = 2,
    Min = 3,
    Max = 4,
}

bitflags! {
    pub struct ColorComponentFlags: u32 {
        const R = 0x0000_0001;
        const G = 0x0000_0002;
        const B = 0x0000_0004;
        const A = 0x0000_0008;
        const RGBA = Self::R.bits | Self::G.bits | Self::B.bits  | Self::A.bits;
        const ALL = Self::R.bits | Self::G.bits | Self::B.bits  | Self::A.bits;
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ColorBlendAttachmentState {
    Disabled,
    Enabled {
        src_color_blend_factor: BlendFactor,
        dst_color_blend_factor: BlendFactor,
        color_blend_op: BlendOp,
        src_alpha_blend_factor: BlendFactor,
        dst_alpha_blend_factor: BlendFactor,
        alpha_blend_op: BlendOp,
        color_write_mask: ColorComponentFlags,
    },
}

impl ColorBlendAttachmentState {
    pub const DISABLED: ColorBlendAttachmentState = ColorBlendAttachmentState::Disabled;
    pub const ALPHA_BLENDING: ColorBlendAttachmentState = ColorBlendAttachmentState::Enabled {
        color_blend_op: BlendOp::Add,
        src_color_blend_factor: BlendFactor::SrcAlpha,
        dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
        alpha_blend_op: BlendOp::Add,
        src_alpha_blend_factor: BlendFactor::One,
        dst_alpha_blend_factor: BlendFactor::Zero,
        color_write_mask: ColorComponentFlags::ALL,
    };
}

impl Default for ColorBlendAttachmentState {
    fn default() -> Self {
        ColorBlendAttachmentState::Disabled
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ColorBlendAttachments<'a> {
    All(&'a ColorBlendAttachmentState),
    Separate(&'a [ColorBlendAttachmentState]),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ColorBlendState<'a> {
    pub logic_op: Option<LogicOp>,
    pub attachments: ColorBlendAttachments<'a>,
    pub blend_constants: [NotNan<f32>; 4],
}

impl<'a> ColorBlendState<'a> {
    pub const DISABLED: ColorBlendState<'static> = ColorBlendState {
        attachments: ColorBlendAttachments::All(&ColorBlendAttachmentState::Disabled),
        blend_constants: [unsafe { mem::transmute(0.0f32) }; 4],
        logic_op: None,
    };

    pub const ALPHA_BLENDING: ColorBlendState<'static> = ColorBlendState {
        attachments: ColorBlendAttachments::All(&ColorBlendAttachmentState::ALPHA_BLENDING),
        blend_constants: [unsafe { mem::transmute(0.0f32) }; 4],
        logic_op: None,
    };
}

/*#[derive(Copy, Clone)]
pub struct GraphicsPipelineCreateInfo<'a, 'b, B: Backend> {
    /// Shaders
    pub shader_stages: GraphicsShaderStages<'a, 'b, B>,
    pub viewport_state: ViewportState<'b>,
    pub rasterization_state: RasterisationState,
    pub multisample_state: MultisampleState,
    pub depth_stencil_state: DepthStencilState,
    pub input_assembly_state: InputAssemblyState,
    pub color_blend_state: ColorBlendState<'b>,
    //pub dynamic_state: DynamicStateFlags,
}*/

pub struct GraphicsPipeline {

}


impl GraphicsPipeline {
    pub fn new() {

    }

    pub fn builder() -> GraphicsPipelineBuilder {
        GraphicsPipelineBuilder {

        }
    }
}

pub struct GraphicsPipelineBuilder {

}

impl GraphicsPipelineBuilder {

    pub fn with_color_blend_state(mut self, color_blend_state: ColorBlendState) -> Self {
        self
    }


    pub fn set_rasterization_state(mut self, rasterization_state: vk::PipelineRasterizationStateCreateInfo)
}