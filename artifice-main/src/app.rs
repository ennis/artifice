use artifice::model::{Atom, Composition, NodeId, Node};
use eframe::{egui, epi};
use std::{fs::File, io::Write, ops::Deref};

struct AddPropertyWindowState {
    target: NodeId,
    name: String,
    type_id: String,
}

impl Default for AddPropertyWindowState {
    fn default() -> Self {
        AddPropertyWindowState {
            target: Default::default(),
            name: "".to_string(),
            type_id: "".to_string(),
        }
    }
}

pub struct TemplateApp {
    comp: Composition,
    add_property_window: Option<AddPropertyWindowState>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        TemplateApp {
            comp: Composition::new(),
            add_property_window: None,
        }
    }
}

pub enum AppAction {
    RenameNode {
        node: NodeId,
        name: Atom,
    },
    RenameProperty {
        node: NodeId,
        property: Atom,
        name: Atom,
    },
}

impl TemplateApp {
    fn show_node_ui(&mut self, node: &Node, ui: &mut egui::Ui, actions: &mut Vec<AppAction>)
    {
        let label = if node.name().is_empty() {
            "<root>".to_string()
        } else {
            node.name().to_string()
        };

        egui::CollapsingHeader::new(label).show(ui, move |ui| {

            let mut name_edit = node.name().to_string();

            ui.heading("Properties:");
            ui.label("Name:");
            if ui.text_edit_singleline(&mut name_edit).changed() {
                /*actions.push(AppAction::RenameNode {
                    node: node,
                    name: name_edit.into(),
                })*/
            }
            ui.end_row();

            for prop in node.properties() {
                ui.label(prop.name().deref());
                ui.label(
                    egui::Label::new(format!("({})", prop.type_id().deref()))
                        .italics()
                        .small(),
                );
                ui.end_row();
            }

            for child in node.children() {
                self.show_node_ui(child, ui, actions);
                ui.end_row();
            }

            ui.separator();

            if ui.button("Add property...").clicked() {
                self.add_property_window = Some(AddPropertyWindowState {
                    target: node_id,
                    .. Default::default()
                });
            }

            if ui.button("Add child...").clicked() {
                self.comp.create_node(node_id, "node".into());
            }
        });
        ui.end_row();
    }

    fn show_add_property_ui(&mut self, ctx: &egui::CtxRef) {
        let mut open = self.add_property_window.is_some();
        egui::Window::new("Add property")
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Grid::new("Add property").show(ui, |ui| {
                    if let Some(ref mut state) = self.add_property_window {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut state.name);
                        ui.end_row();
                        ui.label("Type:");
                        ui.text_edit_singleline(&mut state.type_id);
                        ui.end_row();
                        if ui.button("Add").clicked() {
                            self.comp
                                .node_mut(state.target)
                                .unwrap()
                                .add_property(Atom::from(state.name.as_str()), Atom::from(state.type_id.as_str()));
                        }
                    }
                });
            });
        if !open {
            self.add_property_window = None;
        }
    }
}

impl epi::App for TemplateApp {
    fn name(&self) -> &str {
        "artifice-name"
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let root = self.comp.root_id();

            let mut actions = Vec::new();
            self.show_node_ui(root, ui, &mut actions);
            self.show_add_property_ui(ctx);

            if ui.button("Save to JSON...").clicked() {
                if let Ok(Some(path)) = native_dialog::FileDialog::new().show_save_single_file() {
                    let json = self.comp.to_json();
                    let mut file = File::create(path).unwrap();
                    file.write(json.to_string().as_ref());
                }
            }

            /*// The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("egui template");
            ui.hyperlink("https://github.com/emilk/egui_template");
            ui.add(egui::github_link_file!(
                "https://github.com/emilk/egui_template/blob/master/",
                "Source code."
            ));
            egui::warn_if_debug_build(ui);*/
        });

        /* if false {
            egui::Window::new("Window").show(ctx, |ui| {
                ui.label("Windows can be moved by dragging them.");
                ui.label("They are automatically sized based on contents.");
                ui.label("You can turn on resizing and scrolling if you like.");
                ui.label("You would normally chose either panels OR windows.");
            });
        }*/
    }
}
