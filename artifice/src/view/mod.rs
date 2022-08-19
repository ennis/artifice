//mod toolbar;

use crate::model::{Document, Node, Param, Path};
use kyute::{
    cache, composable,
    shell::{
        text::{Attribute, FontFamily, FontStyle, FormattedText},
        winit::window::WindowBuilder,
    },
    text::FormattedTextExt,
    theme,
    widget::{
        drop_down, grid,
        grid::{GridTemplate, SHOW_GRID_LAYOUT_LINES},
        Action, Button, ColumnHeaders, DropDown, Flex, Grid, Image, Label, Menu, MenuItem, Null, Orientation, Shortcut,
        TableRow, TableView, TableViewParams, Text, WidgetPod,
    },
    Color, Data, Font, Length, State, UnitExt, Widget, Window,
};
use std::{convert::TryFrom, fmt, fmt::Formatter, fs, sync::Arc};

const LABEL_COLUMN: &str = "label";
const ADD_COLUMN: &str = "add";
const DELETE_COLUMN: &str = "delete";
const VALUE_COLUMN: &str = "value";

/// Attribute row
#[composable]
pub fn attribute_row(attribute: &Param) -> TableRow<i64> {
    // format attribute name
    let attr_name = attribute.path.name();
    let mut label = FormattedText::new(attr_name.as_ref());
    if let Some(p) = attr_name.rfind(':') {
        label.add_attribute(0..p + 1, Attribute::Color(theme::palette::GREY_50.with_alpha(0.5)));
    }

    let mut row = TableRow::new(attribute.id, Text::new(label));
    row.add_cell(1, Label::new(attribute.ty.display_glsl().to_string()));
    row.add_cell(2, Label::new(format!("{:?}", attribute.value)));
    row
}

// Takes a single (wrapper) object:
// - points to the element in the document being edited
// - on edit, clone the document, and modify the copy

// node.changed()   returns whether there are pending modifications to the thing
// node.edit_xxx()   to
//

/// Node view.
#[composable]
pub fn node_row(node: &Node) -> TableRow<i64> {
    let label = if node.path.is_root() {
        FormattedText::from("<root node>").attribute(.., FontStyle::Italic)
    } else {
        FormattedText::from(node.name().to_string())
    };

    let mut row = TableRow::new(node.id, Text::new(label));

    for attr in node.attributes.values() {
        cache::scoped(attr.id, || {
            row.add_row(attribute_row(attr));
        });
    }

    for node in node.children.values() {
        cache::scoped(node.id, || {
            row.add_row(node_row(node));
        });
    }

    // format name
    /*let path = node.path.to_string();
    let last_sep = path.rfind('/').unwrap();
    let path_text = FormattedText::from(path)
        .attribute(0..=last_sep, Attribute::Color(Color::new(0.7, 0.7, 0.7, 1.0)))
        .attribute(.., Attribute::FontSize(17.0))
        .attribute(.., FontFamily::new("Cambria"))
        .attribute(.., FontStyle::Italic);*/

    row
}

/// Root document view.
#[composable(live_literals)]
pub fn document_window_contents(document: &Document) -> impl Widget + Clone {
    trace!("document_window_contents");

    let root_row = node_row(&document.root);

    let table_params = TableViewParams {
        selection: None,
        template: GridTemplate::try_from("auto / 200px 200px 1fr").unwrap(),
        column_headers: Some(
            ColumnHeaders::new()
                .add(Text::new("Name"))
                .add(Text::new("Type"))
                .add(Text::new("Value")),
        ),
        main_column: 0,
        rows: vec![root_row],
        row_indent: Length::Dip(20.0),
        resizeable_columns: false,
        reorderable_rows: false,
        reorderable_columns: false,
        background: theme::palette::GREY_800.into(),
        alternate_background: theme::palette::GREY_700.into(),
        row_separator_width: Default::default(),
        column_separator_width: Default::default(),
        row_separator_background: Default::default(),
        column_separator_background: Default::default(),
        selected_style: Default::default(),
    };

    let table = TableView::new(table_params);

    let mut grid = Grid::with_template("auto auto / 1fr");
    //let toolbar = Toolbar::new()
    //    .text_icon_button("Create", "data/icons/file_new.png")
    //   .text_icon_button("Open", "data/icons/file_folder.png")
    //    .text_icon_button("Save", "data/icons/file_tick.png");
    //grid.insert(toolbar);
    grid.insert(table);
    Arc::new(grid)
}

/// Main menu bar.
#[composable]
pub fn main_menu_bar(document: &Document) -> Menu {
    let file_new = Action::with_shortcut(Shortcut::from_str("Ctrl+N"));
    let file_open = Action::with_shortcut(Shortcut::from_str("Ctrl+O"));
    let file_save = Action::with_shortcut(Shortcut::from_str("Ctrl+S"));
    let file_save_as = Action::with_shortcut(Shortcut::from_str("Ctrl+Shift+S"));
    let file_quit = Action::with_shortcut(Shortcut::from_str("Alt+F4"));
    let edit_undo = Action::with_shortcut(Shortcut::from_str("Ctrl+Z"));
    let edit_redo = Action::with_shortcut(Shortcut::from_str("Ctrl+Y"));

    if file_new.triggered() {
        tracing::warn!("File>New unimplemented");
    }
    if file_open.triggered() {
        tracing::warn!("File>Open unimplemented");
    }
    if file_save.triggered() {
        tracing::warn!("File>Save unimplemented");
    }
    if file_save_as.triggered() {
        tracing::warn!("File>Save As unimplemented");
    }
    if file_quit.triggered() {
        tracing::warn!("File>Quit unimplemented");
    }

    if edit_undo.triggered() {
        tracing::warn!("Edit>Undo unimplemented");
    }
    if edit_redo.triggered() {
        tracing::warn!("Edit>Redo unimplemented");
    }

    let mut file_menu = Menu::new(vec![
        MenuItem::new("New", file_new),
        MenuItem::new("Open...", file_open),
        MenuItem::new("Save", file_save),
        MenuItem::new("Save as...", file_save_as),
        MenuItem::separator(),
        MenuItem::new("Quit", file_quit),
    ]);
    let mut edit_menu = Menu::new(vec![MenuItem::new("Undo", edit_undo), MenuItem::new("Redo", edit_redo)]);
    let menu_bar = Menu::new(vec![
        MenuItem::submenu("File", file_menu),
        MenuItem::submenu("Edit", edit_menu),
    ]);

    menu_bar
}

/// Native window displaying a document.
#[composable]
pub fn document_window(document: &Document) -> Window {
    //
    tracing::trace!("document_window");
    let menu_bar = main_menu_bar(document);

    // TODO document title
    Window::new(
        WindowBuilder::new().with_title("Document"),
        document_window_contents(document),
        Some(menu_bar),
    )
}

fn try_open_document() -> anyhow::Result<Document> {
    let xml = fs::read_to_string("test.xml")?;
    Document::from_xml(&xml)
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
