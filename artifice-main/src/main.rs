mod app;

use std::{mem, path::Path, ptr};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use artifice::core::{RenderNode, RenderContext, InputResourceDesc, DescribeCtx, RenderError, ParamType};
use artifice::model::Composition;

//--------------------------------------------------------------------------------------------------
// Render node examples

/*/// Image loader: takes a path to an image file and loads it into an image.
struct ImageLoadRenderNode {}

impl RenderNode for ImageLoadRenderNode {

    fn describe(&self, ctx: &mut DescribeCtx) {
        // this node has one parameter: the path to the image file...
        ctx.declare_parameter("file_path", ParamType::String);

        // ... and produces one image output: the loaded image.
        ctx.declare_output_image("loaded_image");
    }



    fn render(&self, ctx: &mut RenderContext) -> Result<(), RenderError> {
        todo!()
    }
}


/// Scene renderer: takes a scene as input and renders it
struct SceneRenderNode {}

impl RenderNode for SceneRenderNode {
    fn describe(&self, ctx: &mut DescribeCtx) {
        // the "object filter", which defines which objects we need to render in the scene
        // by default, this is "*", which means everything, but it's possible to filter
        // so that only objects with certain properties (materials?) are rendered
        ctx.declare_parameter("object_filter", ParamType::String);

        // G-buffer outputs
        ctx.declare_input_output_image("depth");
        ctx.declare_input_output_image("colors");
        ctx.declare_input_output_image("normals");
        ctx.declare_input_output_image("diffuse");
    }

    fn render(&self, context: &mut RenderContext) -> Result<(), RenderError> {
        todo!()
    }
}*/


fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::ACTIVE)
        .init();

    let app = app::TemplateApp::default();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
