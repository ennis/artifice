use crate::data::{
    atom::{make_unique_name, Atom},
    node::{Node, NodeList},
};
use druid::{
    lens,
    text::{
        format::{Formatter, Validation, ValidationError},
        Selection,
    },
    widget::{Button, Controller, CrossAxisAlignment, Flex, Label, List, TextBox, ValueTextBox},
    Data, Env, Event, EventCtx, Insets, LensExt, Widget, WidgetExt,
};

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
