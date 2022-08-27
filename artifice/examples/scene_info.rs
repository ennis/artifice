use crate::future::FutureExt;
use artifice::{
    eval::{
        imaging::{
            DeviceComputeImageResult, ImageInputRequest, OpImaging, OpImagingCtx, PxSizeI, RegionOfDefinition,
            RequestWindow, TiPoint, TiRect, TiSize,
        },
        EvalError, Evaluation,
    },
    model::{metadata, Document, Node, Path, Value},
    operators::register_builtin_operators,
};
use async_trait::async_trait;
use futures::future;
use kyute::{graal, shell::application::Application};
use kyute_common::{Atom, Rect};
use parking_lot::Mutex;
use std::fs;
use tracing_subscriber::{layer::SubscriberExt, Layer};

fn evaluate_document(document: &Document) {
    // find the display node
    let mut display_image = None;
    for node in document.root().children.values() {
        if let Some(op) = node.metadata(metadata::OPERATOR) {
            if &*op == "display" {
                //display_node = Some(node.path.clone());
                display_image = node
                    .attribute(&Atom::from("input:image"))
                    .and_then(|v| v.connection.clone())
            }
        }
    }
    if let Some(display_image) = display_image {
        // evaluate the input of the display node
        // spin an evalctx
        let device = Application::instance().gpu_device().clone();
        let mut eval = Evaluation::new(device, document.clone());
        let result = eval.device_evaluate_image(
            &display_image,
            0.0,
            &RequestWindow {
                roi: TiRect::new(TiPoint::zero(), TiSize::new(1280.0, 720.0)),
                resolution: PxSizeI::new(1280, 720),
            },
        );

        match result {
            Ok(image) => {}
            Err(err) => {
                eprintln!("failed to evaluate document: {}", err)
            }
        }
    }
}

fn try_open_document() -> anyhow::Result<Document> {
    let xml = fs::read_to_string("data/networks/simple.xml")?;
    let document = Document::from_xml(&xml)?;
    Ok(document)
}

fn main() {
    let subscriber = tracing_subscriber::Registry::default().with(
        tracing_tree::HierarchicalLayer::new(4)
            .with_bracketed_fields(true)
            .with_filter(tracing_subscriber::filter::EnvFilter::from_default_env()),
    );
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _guard = runtime.enter();

    let app = Application::instance();
    register_builtin_operators();
    let document = try_open_document().unwrap();
    eprintln!("{:?}", document);
    evaluate_document(&document);
}
