use crate::application::{Application, Component};
use crate::geom::{GeometryCache, GeometrySources};
use crate::material::StandardViewportMaterial;
use crate::scene::Scene;
use crate::util::MessageBus;
use artifice_macros::topic;
use slotmap::SlotMap;
use std::cell::RefCell;
use std::rc::Rc;

slotmap::new_key_type! {
    pub struct DocumentId;
    pub struct SceneId;
    pub struct MaterialId;
}

#[topic(DocumentChanges)]
pub trait DocumentChangeListener {
    fn scene_added(&mut self, id: DocumentId, doc: &Document, scene: SceneId) {}
}

pub struct Document {
    bus: MessageBus,
    events: DocumentChanges,
    pub scenes: SlotMap<SceneId, Scene>,
    pub materials: SlotMap<MaterialId, StandardViewportMaterial>,
    pub geom_cache: GeometryCache,
    pub geom_sources: GeometrySources,
}

impl Document {
    pub fn new() -> Document {
        let bus = MessageBus::new();
        let events = DocumentChanges::publisher(&bus);
        Document {
            bus,
            events,
            scenes: SlotMap::with_key(),
            materials: SlotMap::with_key(),
            geom_cache: GeometryCache::new(),
            geom_sources: GeometrySources::new(),
        }
    }

    pub fn message_bus(&self) -> &MessageBus {
        &self.bus
    }
}

/// The component that manages all open documents.
pub struct Documents {
    events: OpenDocumentsChanges,
    docs: SlotMap<DocumentId, Document>,
}

impl Documents {
    pub fn new(app: &Application) -> Rc<RefCell<Documents>> {
        let d = Documents {
            events: OpenDocumentsChanges::publisher(app.message_bus()),
            docs: SlotMap::with_key(),
        };
        Rc::new(RefCell::new(d))
    }

    pub fn new_document(&mut self) -> DocumentId {
        let id = self.docs.insert(Document::new());
        let doc = self.docs.get(id).unwrap();
        self.events.document_opened(id, doc);
        id
    }
}

impl Component for Documents {}

#[topic(OpenDocumentsChanges)]
pub trait OpenDocumentsChangeListener {
    fn document_opened(&mut self, id: DocumentId, doc: &Document) {}
    fn document_closed(&mut self, id: DocumentId, doc: &Document) {}
}
