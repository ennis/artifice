use crate::application::{Application, Component};
use crate::geom::{GeometryCache, GeometrySources};
use crate::material::StandardViewportMaterial;
use crate::scene::Scene;
use crate::util::MessageBus;
use crate::util::model::Data;
use anyhow::{Error, Result};
use artifice_macros::topic;
use slotmap::SlotMap;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

slotmap::new_key_type! {
    pub struct DocumentId;
    pub struct SceneId;
    pub struct MaterialId;
}

#[topic(DocumentChanges)]
pub trait DocumentChangeListener {

    /// The name of the document has changed.
    #[allow(unused_variables)]
    fn name_changed(&mut self, doc: &Document) {}

    /// A scene was added to the document.
    #[allow(unused_variables)]
    fn scene_added(&mut self, id: DocumentId, doc: &Document, scene: SceneId) {}
}

pub struct Document {
    bus: MessageBus,
    events: DocumentChanges,
    unsaved_changes: bool,

    pub name: String,
    pub path: Option<PathBuf>,
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
            name: "Unnamed".to_string(),
            path: None,
            unsaved_changes: true,
            bus,
            events,
            scenes: SlotMap::with_key(),
            materials: SlotMap::with_key(),
            geom_cache: GeometryCache::new(),
            geom_sources: GeometrySources::new(),
        }
    }

    pub fn from_gltf<P: AsRef<Path>>(path: P) -> Result<Document> {
        // load geometries
        let mut doc = Document::new();
        doc.path = Some(path.as_ref().to_path_buf());
        crate::gltf::load_gltf(path.as_ref(), &mut doc)?;
        Ok(doc)
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.unsaved_changes
    }

    pub fn message_bus(&self) -> &MessageBus {
        &self.bus
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
        // TODO this is a bit much, maybe the methods on the publisher
        // should not be &mut ?
        self.events.name_changed(self);
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
    #[allow(unused_variables)]
    fn document_opened(&mut self, id: DocumentId, doc: &Document) {}
    #[allow(unused_variables)]
    fn document_closed(&mut self, id: DocumentId, doc: &Document) {}
}
