use anyhow::Context;
use artifice::model::{make_unique_name, Atom, Network, Node};
use druid::{
    lens,
    commands::{REDO, UNDO},
    text::{
        format::{Formatter, Validation, ValidationError},
        Selection,
    },
    widget::{
        Button, Controller, CrossAxisAlignment, Flex, Label, List, MainAxisAlignment, TextBox,
        ValueTextBox,
    },
    AppDelegate, Command, Data, DelegateCtx, Env, Event, EventCtx, FileDialogOptions, FileInfo,
    FileSpec, Handled, Insets, Lens, LensExt, LocalizedString, MenuDesc, MenuItem, SysMods, Target,
    UpdateCtx, Widget, WidgetExt,
};
use std::{fs::File, path::Path, sync::Arc};
use tracing::{error, info};

/// Application state
#[derive(Clone, Data)]
pub struct AppData {
    /// Network being edited
    network: Network,

    /// Path to the file being edited, empty if not saved yet
    #[data(ignore)]
    current_file_info: Option<FileInfo>,
}

impl AppData {
    /// Creates a new, blank application state.
    pub fn new() -> AppData {
        AppData {
            network: Network::new(),
            current_file_info: None,
        }
    }

    /// Saves the current network to the specified file.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(path).context("Failed to open file for saving")?;
        let json = self.network.to_json();
        serde_json::to_writer_pretty(&mut file, &json).context("failed to write JSON")?;
        Ok(())
    }

    /// Returns a lens to the network.
    pub fn network_lens() -> impl Lens<Self, Network> {
        druid::lens!(Self, network)
    }
}

const FILE_TYPES: &[FileSpec] = &[FileSpec::new("Artifice JSON", &["json"])];

fn get_default_file_dialog_options() -> FileDialogOptions {
    FileDialogOptions::new().allowed_types(FILE_TYPES.to_vec())
}

/// Undo/redo controller.
pub struct UndoRedoController {
    /// Stack of previous states: the top is the current state
    stack: Vec<Network>,

    /// Position in the stack, starting from the top (0 is last pushed state)
    position: usize,
}

impl UndoRedoController {
    pub fn new() -> UndoRedoController {
        UndoRedoController {
            stack: vec![Network::new()],
            position: 0,
        }
    }
}

/// Gives a name to the changes (actions) that a child widget performs on some data.
pub struct ActionWrapper {
    action_name: String,
}

impl ActionWrapper {
    pub fn new(name: impl Into<String>) -> ActionWrapper {
        ActionWrapper {
            action_name: name.into(),
        }
    }
}

impl<T: Data, W: Widget<T>> Controller<T, W> for ActionWrapper {
    fn update(&mut self, child: &mut W, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        child.update(ctx, old_data, data, env);
        if !old_data.same(data) {
            // TODO expose that to the UI. via commands?
            tracing::info!("Command: {}", self.action_name);
        }
    }
}

impl<W: Widget<Network>> Controller<Network, W> for UndoRedoController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut Network,
        env: &Env,
    ) {
        // handle undo and redo commands
        match event {
            druid::Event::Command(cmd) if cmd.is(UNDO) => {
                if self.position == 0 {
                    tracing::warn!("nothing to undo");
                } else {
                    self.position -= 1;
                    *data = self.stack[self.position].clone();
                }
            }
            druid::Event::Command(cmd) if cmd.is(REDO) => {
                if self.position == self.stack.len() - 1 {
                    // already at the top of the stack
                    tracing::warn!("nothing to redo");
                } else {
                    self.position += 1;
                    *data = self.stack[self.position].clone();
                }
            }
            _ => {
                child.event(ctx, event, data, env);
            }
        }
    }

    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut UpdateCtx,
        old_data: &Network,
        data: &Network,
        env: &Env,
    ) {
        child.update(ctx, old_data, data, env);

        // If the data has changed, record the old data in the undo stack
        if !old_data.same(data) && !self.stack[self.position].same(data) {
            tracing::trace!("pushing undo entry");
            // destroy edits after this one
            self.stack.drain((self.position + 1)..);
            self.stack.push(data.clone());
            self.position = self.stack.len() - 1;
        }
    }
}

/// Application delegate.
pub struct Delegate;

impl AppDelegate<AppData> for Delegate {
    fn command(
        &mut self,
        ctx: &mut DelegateCtx,
        target: Target,
        cmd: &Command,
        data: &mut AppData,
        _env: &Env,
    ) -> Handled {
        if cmd.is(druid::commands::SAVE_FILE_AS) {
            let file_info = cmd.get_unchecked(druid::commands::SAVE_FILE_AS);
            match data.save(file_info.path()) {
                Ok(_) => {
                    // save success, update save path
                    data.current_file_info = Some(file_info.clone());
                    info!("saved to {}", file_info.path().display());
                }
                Err(e) => {
                    // TODO message box instead?
                    error!("error saving file: {:?}", e);
                }
            }
            Handled::Yes
        } else if cmd.is(druid::commands::SAVE_FILE) {
            if let Some(ref file_info) = data.current_file_info {
                // save to the current path
                match data.save(file_info.path()) {
                    Ok(_) => {
                        info!("saved to {}", file_info.path().display());
                    }
                    Err(e) => {
                        // TODO message box instead?
                        error!("error saving file: {:?}", e);
                    }
                }
            } else {
                // not saved, show the file save dialog
                ctx.submit_command(
                    druid::commands::SHOW_SAVE_PANEL
                        .with(get_default_file_dialog_options())
                        .to(target),
                );
            }

            //data.save_to_json().unwrap();
            Handled::Yes
        } else {
            println!("cmd forwarded: {:?}", cmd);
            Handled::No
        }
    }
}

type NodeList = Arc<Vec<Node>>;

/// Formatter that automatically renames the inputted name if it clashes with another node.
struct NodeIdentFormatter;

impl Formatter<Atom> for NodeIdentFormatter {
    fn format(&self, name: &Atom) -> String {
        name.to_string()
    }

    fn validate_partial_input(&self, input: &str, _sel: &Selection) -> Validation {
        Validation::success()
        /*if let Some(unique_name) = make_unique_name(input, self.siblings.iter().map(|n| n.name())) {
            Validation::success().change_text(unique_name)
        } else {
            Validation::success()
        }*/
    }

    fn value(&self, input: &str) -> Result<Atom, ValidationError> {
        Ok(Atom::from(input))
    }
}

/// Controller in charge of ensuring that the name of a node is unique within a list of siblings.
struct NodeRenameController;

impl<W: Widget<(NodeList, Node)>> Controller<(NodeList, Node), W> for NodeRenameController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut (NodeList, Node),
        env: &Env,
    ) {
        let old = data.1.clone();
        child.event(ctx, event, data, env);
        if !old.same(&data.1) {
            data.1.name = make_unique_name(data.1.name.clone(), data.0.iter().map(|n| &n.name));
        }
    }
}

pub fn node_ui(depth: u32) -> impl Widget<(NodeList, Node)> {
    let indent = Insets::new((depth * 10) as f64, 0.0, 0.0, 0.0);

    let mut vbox = Flex::column().cross_axis_alignment(CrossAxisAlignment::Start);

    // name row
    vbox.add_child(
        Flex::row()
            //.must_fill_main_axis(true)
            .with_child(Label::new("Name").padding(indent).fix_width(200.0))
            .with_flex_child(
                ValueTextBox::new(TextBox::new(), NodeIdentFormatter)
                    .lens(lens!(Node, name))
                    .lens(lens!((NodeList, Node), 1))
                    .controller(NodeRenameController)
                    .controller(ActionWrapper::new("rename node"))
                    .expand_width(),
                1.0,
            ),
    );

    // button to add a child
    vbox.add_child(
        Button::new("Add child")
            .on_click(|_, data: &mut (NodeList, Node), _| {
                data.1.add_child("node".into());
            })
            .padding(indent),
    );

    // children
    vbox.add_child(
        List::new(move || node_ui(depth + 1)).lens(druid::lens::Identity.map(
            |(_, node): &(NodeList, Node)| (node.children.clone(), node.children.clone()),
            |(_, node): &mut (NodeList, Node), (_, new): (NodeList, NodeList)| {
                node.children = new;
            },
        )),
    );

    vbox
}

pub fn ui() -> impl Widget<AppData> {
    let mut root = Flex::column();

    root.add_child(
        node_ui(0)
            .lens(druid::lens::Identity.map(
                |net: &Network| (Arc::new(Vec::new()), net.root().clone()),
                |net: &mut Network, (_, new): (_, Node)| {
                    net.root = new;
                }
            ))
            .controller(UndoRedoController::new())
            .lens(AppData::network_lens())
            .fix_width(600.0),
    );
    root
    //root.debug_paint_layout()
}

pub fn application_menu() -> MenuDesc<AppData> {
    MenuDesc::new(LocalizedString::new("artifice").with_placeholder("artifice"))
        .append(
            MenuDesc::new(LocalizedString::new("artifice.file-menu").with_placeholder("File"))
                .append(
                    MenuItem::new(
                        LocalizedString::new("artifice.open").with_placeholder("Open..."),
                        druid::commands::SHOW_OPEN_PANEL.with(get_default_file_dialog_options()),
                    )
                    .hotkey(SysMods::Cmd, "o"),
                )
                .append(
                    MenuItem::new(
                        LocalizedString::new("artifice.save").with_placeholder("Save"),
                        druid::commands::SAVE_FILE,
                    )
                    .hotkey(SysMods::Cmd, "s"),
                )
                .append(
                    MenuItem::new(
                        LocalizedString::new("artifice.save-as").with_placeholder("Save as..."),
                        druid::commands::SHOW_SAVE_PANEL.with(get_default_file_dialog_options()),
                    )
                    .hotkey(SysMods::CmdShift, "s"),
                )
                .append_separator()
                .append(MenuItem::new(
                    LocalizedString::new("artifice.quit").with_placeholder("Quit"),
                    druid::commands::QUIT_APP,
                )),
        )
        .append(
            MenuDesc::new(LocalizedString::new("artifice.edit-menu").with_placeholder("Edit"))
                .append(druid::platform_menus::common::undo())
                .append(druid::platform_menus::common::redo())
                .append_separator()
                .append(druid::platform_menus::common::cut())
                .append(druid::platform_menus::common::copy())
                .append(druid::platform_menus::common::paste())
                .append(MenuItem::new(
                    LocalizedString::new("artifice.redo").with_placeholder("Redo"),
                    druid::commands::REDO,
                )),
        )
}
