use crate::document::Document;
use crate::document::DocumentChangeListener;
use crate::render::gl::api::gl;
use crate::ui::common::platform::{OpenGlDrawContext, PlatformWindow};
use crate::ui::common::EventResult;
use crate::ui::common::WindowCtx;
use crate::ui::common::WindowEventTarget;
use crate::ui::kyute::renderer::{Painter, Renderer};

use crate::document::DocumentId;
use crate::document::SceneId;

use crate::ui::kyute;
use crate::ui::kyute::Widget;
use anyhow::Result;
use log::trace;
use std::cell::Cell;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use winit::event::ElementState;
use winit::event::MouseButton;
use winit::event::WindowEvent;
use winit::window::{WindowBuilder, WindowId};

const CMD_MENU_FILE_OPEN: u32 = 0x101;
const CMD_MENU_FILE_EXIT: u32 = 0x100;

/// Listens to changes in a document, updates the window in return.
struct DocumentViewWrapper {
    changed_title: Option<String>,
}

impl DocumentChangeListener for DocumentViewWrapper {
    fn name_changed(&mut self, _doc: &Document) {
        trace!("document name changed");
    }

    fn scene_added(&mut self, _id: DocumentId, _doc: &Document, _scene: SceneId) {
        trace!("scene added");
    }
}

fn document_window_title_bar(doc: &Document) -> String {
    let unsaved = if doc.has_unsaved_changes() { "(*)" } else { "" };
    format!("Artifice - {} {}", doc.name, unsaved)
}

pub fn open_document_window(
    ctx: &mut WindowCtx,
    docs_counter: Rc<Cell<usize>>,
    path: Option<&Path>,
) -> Result<()> {
    let doc = if let Some(path) = path {
        Document::from_gltf(path)?
    } else {
        Document::new()
    };

    let window = PlatformWindow::new(
        ctx,
        WindowBuilder::new().with_title(document_window_title_bar(&doc)),
        true,
    )?;

    let ui_renderer = Renderer::new(ctx);

    let handler = DocumentWindowHandler {
        doc,
        docs_counter,
        ui_cache: kyute::Cache::new(),
        window,
        wrap: Rc::new(RefCell::new(DocumentViewWrapper {
            changed_title: None,
        })),
        size: (0.0, 0.0),
        ui_renderer,
    };
    ctx.add_window(handler);

    Ok(())
}
