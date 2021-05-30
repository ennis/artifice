use tracing::trace;
use thiserror::Error;
use graal::vk;

/// Metadata about an image
pub struct ImageMetadata {
    width: u32,
    height: u32,
    format: vk::Format,
}

pub enum InputResourceKind {
    /// Describes a buffer input
    Buffer,
    /// Describes an image input
    Image,
    /// Describes a scene (collection of objects with associated geometry and attributes)
    Scene,
    /// For variadic parameters, there should be only one
    Ellipsis,
}

/// Describes an input of a node.
pub struct InputResourceDesc {

    /// The name of the input.
    pub name: String,

    /// Whether this input is required.
    pub required: bool,

    /// The type of resource.
    pub kind: InputResourceKind
}

pub struct InputResources {
}

impl InputResources {
    ///
    pub fn get_image_metadata(&self, name: &str) -> Option<ImageMetadata> {
        todo!()
    }
}

/// Context passed during rendering
pub struct RenderContext<'a> {
    /// Current frame
    frame: graal::Frame<'a>,
}

impl<'a> RenderContext<'a> {
    pub fn get_input_image(&self, name: &str) -> Option<graal::ImageInfo> {
        todo!()
    }

    pub fn get_output_image(&self, name: &str) -> Option<graal::ImageInfo> {
        todo!()
    }

    pub fn frame(&self) -> &graal::Frame<'a> {
        &self.frame
    }

    /// Returns the value of a parameter
    pub fn get_value(&self, param_name: &str) -> Option<&str> {
        todo!()
    }
}

pub struct DescribeCtx {


}

/// The type of a node parameter
#[derive(Copy,Clone,Debug,Eq,PartialEq,Hash)]
pub enum ParamType {
    Integer,
    Float,
    String,
}

impl DescribeCtx {

    pub fn declare_input_image(&mut self, name: &str) {
        trace!("declare_input_image: {}", name);
        todo!()
    }

    pub fn declare_parameter(&mut self, name: &str, ty: ParamType) {
        trace!("declare_parameter: {}", name);
        todo!()
    }

    pub fn declare_output_image(&mut self, name: &str) {
        trace!("declare_parameter: {}", name);
        todo!()
    }
}

#[derive(Error, Debug)]
pub enum RenderError {
    #[error("input `{0}` not found")]
    InputNotFound(String),
    #[error("parameter `{0}` not found")]
    ParameterNotFound(String),
    #[error("parameter `{0}` not found")]
    OutputNotFound(String),
}

/// Trait describing the behavior of a render node.
pub trait RenderNode {
    /// Describes this render node to the evaluator.
    ///
    /// This method communicates to the evaluator the inputs of the node, the outputs that it produces,
    /// and its parameters.
    fn describe(&self, ctx: &mut DescribeCtx);


    /// Renders the node.
    fn render(&self, context: &mut RenderContext) -> Result<(), RenderError>;
}