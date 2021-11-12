use crate::{
    data::{
        atom::{make_unique_name, Atom},
        node::{Node, NodeList},
        property::Property,
        AppData, AsBoolLens, AsNumberLens, AsStringLens,
    },
    widgets::tree::TreeNodeData,
};
use druid::{
    lens,
    lens::Identity,
    text::{
        Formatter, Validation, ValidationError, Selection,
    },
    widget::{
        Button, Checkbox, Controller, CrossAxisAlignment, Flex, Label, List, Slider, Spinner,
        Stepper, TextBox, ValueTextBox, ViewSwitcher,
    },
    Data, Env, Event, EventCtx, FontDescriptor, FontFamily, FontStyle, Insets, Lens, LensExt,
    LocalizedString, Menu, MenuItem, Selector, Widget, WidgetExt,
};
use std::sync::Arc;

const REMOVE_NODE: Selector = Selector::new("artifice.node.remove");
const ADD_CHILD_NODE: Selector = Selector::new("artifice.node.add-child");

/// Formatter that automatically renames the inputted name if it clashes with another node.
struct NodeIdentFormatter;

impl Formatter<Atom> for NodeIdentFormatter {
    fn format(&self, name: &Atom) -> String {
        name.to_string()
    }

    fn validate_partial_input(&self, input: &str, _sel: &Selection) -> Validation {
        Validation::success()
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

/// UI for a node
pub fn node_ui(depth: u32) -> impl Widget<(NodeList, Node)> {
    let indent = Insets::new((depth * 10) as f64, 0.0, 0.0, 0.0);
    let mut vbox = Flex::column().cross_axis_alignment(CrossAxisAlignment::Start);

    /*ui! {
        VBox {
            Row {
                Label("name"),
                TextEdit(name: \node.name, onChange: |name| { if validate(list, name) { \node.name = name; })
            }
        }
    }*/

    // name row
    vbox.add_child(
        Flex::row()
            .with_child(Label::new("Name").padding(indent).fix_width(200.0))
            .with_flex_child(
                ValueTextBox::new(TextBox::new(), NodeIdentFormatter)
                    .lens(lens!(Node, name))
                    .lens(lens!((NodeList, Node), 1))
                    .controller(NodeRenameController)
                    .expand_width(),
                1.0,
            ),
    );

    // button to add a child
    vbox.add_child(
        Button::new("Add child")
            .on_click(|_, data: &mut (NodeList, Node), _| {
                data.1.add_child("node".into());
                data.1.add_property("thing".into(), "int".into());
                data.1.add_property("thing".into(), "float".into());
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

    // properties
    vbox.add_child(List::new(move || property_ui()).lens(Identity.map(
        |(_, node): &(NodeList, Node)| (node.clone(), node.properties.clone()),
        |(_, node): &mut (NodeList, Node), (_, new_properties): (Node, Arc<Vec<Property>>)| {
            node.properties = new_properties;
        },
    )));

    vbox
}

fn node_context_menu() -> Menu<AppData> {
    Menu::new(LocalizedString::new("Node"))
        .entry(MenuItem::new(LocalizedString::new("Add child")).command(ADD_CHILD_NODE))
        .entry(MenuItem::new(LocalizedString::new("Remove")).command(REMOVE_NODE))
}

/// The widget representing the node in the tree view.
pub fn tree_node_label() -> impl Widget<TreeNodeData<Node>> {
    /// Makes the node context menu appear when right-clicking a node in the tree view
    struct ContextMenuController;

    impl<W> Controller<TreeNodeData<Node>, W> for ContextMenuController
    where
        W: Widget<TreeNodeData<Node>>,
    {
        fn event(
            &mut self,
            child: &mut W,
            ctx: &mut EventCtx,
            event: &Event,
            data: &mut TreeNodeData<Node>,
            env: &Env,
        ) {
            match event {
                Event::MouseDown(ref mouse) if mouse.button.is_right() => {
                    tracing::trace!("context menu");
                    ctx.show_context_menu(node_context_menu(), mouse.window_pos);
                }
                _ => child.event(ctx, event, data, env),
            }
        }
    }

    Label::dynamic(|data: &TreeNodeData<Node>, _| data.node.name.to_string())
        .controller(ContextMenuController)
}

/// UI for a node property (parent node + property)
pub fn property_ui() -> impl Widget<(Node, Property)> {
    #[derive(Copy, Clone, Debug, Data, PartialEq, Eq)]
    enum PropertyGui {
        TextInput,
        Spinner,
        Slider,
        Color,
        FilePath,
        CheckBox,
        None,
    }

    // problem: serde_json::Value is not "Data"
    // -> replace with a Data-compatible version?

    ViewSwitcher::new(
        |prop: &Property, env| {
            let ty_str: &str = &prop.ty;
            match ty_str {
                "float" => PropertyGui::Slider,
                "int" => PropertyGui::Spinner,
                "string" => PropertyGui::TextInput,
                "checkbox" => PropertyGui::CheckBox,
                _ => PropertyGui::None,
            }
        },
        |ty, data, env| match ty {
            PropertyGui::Slider => Slider::new()
                .lens(AsNumberLens)
                .lens(Property::value_lens)
                .boxed(),
            PropertyGui::Spinner => Flex::row()
                .with_child(Stepper::new())
                .with_spacer(4.0)
                .with_child(Label::new(|data: &f64, _: &_| format!("{:.1}", data)))
                .lens(AsNumberLens)
                .lens(Property::value_lens)
                .boxed(),
            PropertyGui::TextInput => TextBox::new()
                .lens(AsStringLens)
                .lens(Property::value_lens)
                .boxed(),
            PropertyGui::CheckBox => Checkbox::new("")
                .lens(AsBoolLens)
                .lens(Property::value_lens)
                .boxed(),
            _ => Label::new(LocalizedString::new("unsupported"))
                .with_font(FontDescriptor::new(FontFamily::SYSTEM_UI).with_style(FontStyle::Italic))
                .boxed(),
        },
    )
    .lens(druid::lens!((Node, Property), 1))
}
