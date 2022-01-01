use crate::model::{Document, ModelPath, Node};
use kyute::{
    composable,
    shell::winit::window::WindowBuilder,
    widget::{Axis, Button, Flex, Text},
    Cache, Key, WidgetPod, Window,
};
use rusqlite::Connection;
use std::sync::Arc;

/// Node view.
#[composable]
pub fn node_item(node: &Node) -> WidgetPod {
    // just a text item with the node name for now
    Text::new(format!("{}({})", node.base.path.to_string(), node.base.id))
}

/// Root document view.
#[composable]
pub fn document_window_contents(#[uncached] document: &mut Document) -> WidgetPod {
    eprintln!("document_window_contents");

    let document_model = document.model();

    let flex_items = {
        // Root nodes
        let mut node_views: Vec<WidgetPod> = Vec::new();
        for (_name, node) in document_model.root.children.iter() {
            Cache::scoped(node.base.id as usize, || node_views.push(node_item(node)))
        }

        // "Add Node" button
        let add_node_button = Button::new("Add Node".to_string());
        if add_node_button.clicked() {
            let name = document_model.root.make_unique_child_name("node");
            document.create_node(ModelPath::root().join(name));
        }

        let mut flex_items = Vec::new();
        flex_items.extend(node_views);
        flex_items.push(add_node_button);
        flex_items
    };

    // enclosing window
    Flex::new(Axis::Vertical, flex_items)
}

/// Native window displaying a document.
#[composable]
pub fn document_window(#[uncached] document: &mut Document) -> WidgetPod {
    // TODO document title
    Window::new(
        WindowBuilder::new().with_title("Document"),
        document_window_contents(document),
    )
}

fn try_open_document() -> anyhow::Result<Document> {
    Ok(Document::open(Connection::open("test.artifice")?)?)
}

/// Application root.
#[composable(uncached)]
pub fn application_root() -> WidgetPod {
    eprintln!("application_root");

    let (mut document, key): (Option<Document>, Key<Option<Document>>) =
        Cache::extract_state(|| None);

    let mut invalidate = false;
    let old_revision: Option<usize> = document.as_ref().map(|doc| doc.revision());

    let widget = if let Some(ref mut document) = document {
        let widget = document_window(document);
        invalidate = Some(document.revision()) != old_revision;
        widget
    } else {
        // create document
        // TODO open file dialog
        let window_contents: WidgetPod = match try_open_document() {
            Ok(new_document) => {
                document = Some(new_document);
                invalidate = true;
                Flex::new(Axis::Vertical, vec![])
            }
            Err(e) => {
                // error message
                Text::new(format!("Could not open file: {}", e))
            }
        };

        Window::new(
            WindowBuilder::new().with_title("No document"),
            window_contents,
        )
    };

    Cache::set_value(key, document, invalidate);
    widget
}
