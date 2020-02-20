use crate::document::{Document, DocumentChangeListener, Documents, DocumentId, SceneId};
use druid_shell::{WinCtx, WindowHandle, WinHandler};
use kyute::paint::RenderContext;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct DocumentViewWrapper {
    doc: DocumentId,
}

impl DocumentChangeListener for DocumentViewWrapper {
    fn scene_added(&mut self, id: DocumentId, doc: &Document, scene: SceneId) {
        eprintln!("DocumentViewWrapper::scene_added")
    }
}

pub struct DocumentWindowHandler {
    /// Reference to the OpenDocuments component
    docs: Rc<RefCell<Documents>>,
    /// Open document counter. When this reaches zero, the application should exit.
    ndocs: Rc<Cell<usize>>,
    wrap: Rc<RefCell<DocumentViewWrapper>>,
    handle: WindowHandle,
    size: (f64, f64),
}

impl DocumentWindowHandler {
    pub fn new(
        docs: Rc<RefCell<Documents>>,
        ndocs: Rc<Cell<usize>>,
        doc: DocumentId,
    ) -> DocumentWindowHandler {
        DocumentWindowHandler {
            docs,
            handle: WindowHandle::default(),
            wrap: Rc::new(RefCell::new(DocumentViewWrapper { doc })),
            size: (0.0, 0.0),
            ndocs,
        }
    }
}


impl WinHandler for DocumentWindowHandler {
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = handle.clone();
    }

    fn paint(&mut self, rctx: &mut RenderContext, wctx: &mut dyn WinCtx) -> bool {
        //unimplemented!()
        false
    }

    fn size(&mut self, width: u32, height: u32, _ctx: &mut dyn WinCtx) {
        let dpi = self.handle.get_dpi();
        let dpi_scale = dpi as f64 / 96.0;
        let width_f = (width as f64) / dpi_scale;
        let height_f = (height as f64) / dpi_scale;
        self.size = (width_f, height_f);
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn destroy(&mut self, ctx: &mut dyn WinCtx) {}
}
