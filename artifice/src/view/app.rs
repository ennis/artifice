//! Root application view
use crate::{
    data::{atom::Atom, network::Network, node::Node, AppData},
    view::node::{node_ui, tree_node_label},
    widgets::tree::{TreeNodeData, TreeView},
};
use anyhow::Context;
use druid::{
    commands::{REDO, UNDO},
    lens,
    text::{
        Formatter, Validation, ValidationError, Selection,
    },
    widget::{
        Button, Controller, CrossAxisAlignment, Flex, Label, List, MainAxisAlignment, TextBox,
        ValueTextBox,
    },
    AppDelegate, BoxConstraints, Command, Data, DelegateCtx, Env, Event, EventCtx,
    FileDialogOptions, FileInfo, FileSpec, Handled, Insets, LayoutCtx, Lens, LensExt, LifeCycle,
    LifeCycleCtx, LocalizedString, Menu, MenuItem, PaintCtx, Size, SysMods, Target, UpdateCtx,
    Widget, WidgetExt, WidgetPod,
};
use std::{fs::File, path::Path, sync::Arc};
use tracing::{error, info};

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

            Handled::Yes
        } else if cmd.is(druid::commands::OPEN_FILE) {
            let file_info = cmd.get_unchecked(druid::commands::OPEN_FILE);
            // TODO if the current document is not saved, show save/discard/cancel dialog
            // TODO if opening fails, don't replace current document
            // TODO
            // load the json
            match data.load(file_info.path()) {
                Ok(_) => {
                    data.current_file_info = Some(file_info.clone());
                    info!("opened {}", file_info.path().display());
                }
                Err(e) => {
                    error!("error opening file: {:?}", e);
                }
            }

            Handled::Yes
        } else {
            println!("cmd forwarded: {:?}", cmd);
            Handled::No
        }
    }
}

pub fn ui() -> impl Widget<AppData> {
    let mut root = Flex::column().cross_axis_alignment(CrossAxisAlignment::Start);

    root.add_child(
        node_ui(0)
            .lens(druid::lens::Identity.map(
                |net: &Network| (Arc::new(Vec::new()), net.root.node.clone()),
                |net: &mut Network, (_, new): (_, Node)| {
                    net.root.node = new;
                },
            ))
            .controller(UndoRedoController::new())
            .lens(AppData::network_lens())
            .fix_width(600.0),
    );

    root.add_child(
        TreeView::new(|| tree_node_label())
            .lens(Network::tree_root_lens())
            .lens(AppData::network_lens())
            .fix_width(600.0),
    );

    //root
    root.debug_widget_id()
}

pub fn application_menu() -> Menu<AppData> {
    Menu::new(LocalizedString::new("artifice").with_placeholder("artifice"))
        .entry(
            Menu::new(LocalizedString::new("artifice.file-menu").with_placeholder("File"))
                .entry(
                    MenuItem::new(
                        LocalizedString::new("artifice.open").with_placeholder("Open..."),
                    )
                    .command(
                        druid::commands::SHOW_OPEN_PANEL.with(get_default_file_dialog_options()),
                    )
                    .hotkey(SysMods::Cmd, "o"),
                )
                .entry(
                    MenuItem::new(LocalizedString::new("artifice.save").with_placeholder("Save"))
                        .command(druid::commands::SAVE_FILE)
                        .hotkey(SysMods::Cmd, "s"),
                )
                .entry(
                    MenuItem::new(
                        LocalizedString::new("artifice.save-as").with_placeholder("Save as..."),
                    )
                    .command(
                        druid::commands::SHOW_SAVE_PANEL.with(get_default_file_dialog_options()),
                    )
                    .hotkey(SysMods::CmdShift, "s"),
                )
                .separator()
                .entry(
                    MenuItem::new(LocalizedString::new("artifice.quit").with_placeholder("Quit"))
                        .command(druid::commands::QUIT_APP),
                ),
        )
        .entry(
            Menu::new(LocalizedString::new("artifice.edit-menu").with_placeholder("Edit"))
                .entry(druid::platform_menus::common::undo())
                .entry(druid::platform_menus::common::redo())
                .separator()
                .entry(druid::platform_menus::common::cut())
                .entry(druid::platform_menus::common::copy())
                .entry(druid::platform_menus::common::paste())
                .entry(
                    MenuItem::new(LocalizedString::new("artifice.redo").with_placeholder("Redo"))
                        .command(druid::commands::REDO),
                ),
        )
}
