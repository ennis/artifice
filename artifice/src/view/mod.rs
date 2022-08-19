use crate::model::Document;
use kyute::{
    cache, composable,
    shell::winit::window::WindowBuilder,
    style::Shape,
    text::FormattedTextExt,
    widget::{Text, WidgetExt},
    UnitExt, Widget, Window,
};
use std::fs;

#[composable]
fn document_window_contents(document: &Document) -> impl Widget {
    Text::new("-- NO SIGNAL --".font_size(40.0).font_family("MS 33558"))
        .centered()
        .frame(100.percent(), 100.percent())
        .background("rgb(10 10 10 / 255)", Shape::rectangle())
}

/// Native window displaying a document.
#[composable]
pub fn document_window(document: &Document) -> Window {
    Window::new(
        WindowBuilder::new().with_title("Document"),
        document_window_contents(document),
        None,
    )
}

fn try_open_document() -> anyhow::Result<Document> {
    let xml = fs::read_to_string("test.xml")?;
    let document = Document::from_xml(&xml)?;
    eprintln!("document: {:?}", document);
    Ok(document)
}

/// Application root.
#[composable]
pub fn application_root() -> impl Widget {
    let document_file_state = cache::state(|| Some(try_open_document().unwrap()));
    let mut doc = document_file_state.take_without_invalidation().unwrap();

    let rev = doc.revision;
    let window = document_window(&doc);
    if doc.revision != rev {
        document_file_state.set(Some(doc));
    } else {
        document_file_state.set_without_invalidation(Some(doc));
    }

    window
}
