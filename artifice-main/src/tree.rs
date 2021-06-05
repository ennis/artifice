// tree widget:
// - hierarchy of

use druid::{
    kurbo::Line, widget::Button, BoxConstraints, Color, Data, Env, Event, EventCtx, LayoutCtx,
    LifeCycle, LifeCycleCtx, PaintCtx, Point, Rect, RenderContext, Size, UpdateCtx, Widget,
    WidgetPod,
};
use std::sync::Arc;

/// A tree node, with methods providing its own label and its children.
/// This is the data expected by the tree widget.
pub trait TreeModel {
    /// Returns how many children are below this node. It could be zero if this is a leaf.
    fn children_count(&self) -> usize;

    /// Returns a reference to the node's child at the given index
    fn get_child(&self, index: usize) -> &Self
    where
        Self: Sized;

    /// Returns a mutable reference to the node's child at the given index
    fn get_child_mut(&mut self, index: usize) -> &mut Self
    where
        Self: Sized;
}

#[derive(Clone, Data)]
pub struct TreeSelectionModel<T: TreeModel + Data> {
    pub selection: Arc<Vec<T>>,
    pub node: T,
}

impl<T: TreeModel + Data> TreeSelectionModel<T> {
    pub fn is_selected(&self) -> bool {
        self.selection
            .iter()
            .find(|&x| x.same(&self.node))
            .is_some()
    }

    pub fn child_data(&self, i: usize) -> TreeSelectionModel<T> {
        TreeSelectionModel {
            selection: self.selection.clone(),
            node: self.node.get_child(i).clone(),
        }
    }
}

pub struct TreeNodeWidget<T: Data + TreeModel, W> {
    expanded: bool,
    toggle: WidgetPod<bool, Box<dyn Widget<bool>>>,
    widget: WidgetPod<T, W>,
    children: Vec<WidgetPod<TreeSelectionModel<T>, Self>>,
    /// Creates child widgets
    closure: Arc<dyn Fn() -> W>,
}

impl<T: Data + TreeModel, W: Widget<T>> TreeNodeWidget<T, W> {
    pub fn new(widget: W, closure: Arc<dyn Fn() -> W>) -> TreeNodeWidget<T, W> {
        let toggle = Button::new("").on_click(|_ctx: &mut _, data: &mut bool, env: &_| {
            *data = !*data;
        });

        TreeNodeWidget {
            expanded: false,
            toggle: WidgetPod::new(Box::new(toggle)),
            widget: WidgetPod::new(widget),
            children: vec![],
            closure,
        }
    }

    fn expand(&mut self, data: &T) {
        // todo
    }
}

impl<T: Data + TreeModel, W: Widget<T>> Widget<TreeSelectionModel<T>> for TreeNodeWidget<T, W> {
    fn event(
        &mut self,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut TreeSelectionModel<T>,
        env: &Env,
    ) {
        let prev_expanded = self.expanded;
        self.toggle.event(ctx, event, &mut self.expanded, env);
        for (i, c) in self.children.iter_mut().enumerate() {
            let mut child_data = data.child_data(i);
            c.event(ctx, event, &mut child_data, env);

            if !child_data.node.same(data.node.get_child(i)) {
                *data.node.get_child_mut(i) = child_data.node;
            }
            data.selection = child_data.selection;
        }

        if prev_expanded != self.expanded {
            if self.expanded {
                // create widgets for new nodes
                for _ in 0..data.node.children_count() {
                    let child = TreeNodeWidget::new((self.closure)(), self.closure.clone());
                    self.children.push(WidgetPod::new(child));
                }
                ctx.children_changed();
            } else {
                // not very efficient, also we lose the "expanded" state...
                self.children.clear();
                ctx.children_changed();
            }
            // expanded or collapsed the node
            tracing::trace!("expanded node");
        } else {
            if !ctx.is_handled() {
                if let Event::MouseUp(_) = event {
                    // toggle selection
                    if data.is_selected() {
                        let node = &data.node;
                        eprintln!("removing from selection");
                        Arc::make_mut(&mut data.selection).retain(|x| !x.same(&node));
                    } else {
                        eprintln!("adding to selection");
                        Arc::make_mut(&mut data.selection).push(data.node.clone());
                    }
                    ctx.request_paint();
                    ctx.set_handled();
                }
            }
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &TreeSelectionModel<T>,
        env: &Env,
    ) {
        self.toggle.lifecycle(ctx, event, &self.expanded, env);
        self.widget.lifecycle(ctx, event, &data.node, env);
        for (i, c) in self.children.iter_mut().enumerate() {
            c.lifecycle(ctx, event, &data.child_data(i), env);
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &TreeSelectionModel<T>,
        data: &TreeSelectionModel<T>,
        env: &Env,
    ) {
        // TODO handle insertion/deletion of children
        // TODO handle external selection changes

        if !old_data.node.same(&data.node) {
            // the node has changed, check if the number of children is the same
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &TreeSelectionModel<T>,
        env: &Env,
    ) -> Size {
        let h = env.get(druid::theme::BASIC_WIDGET_HEIGHT);
        let mut min_w = bc.min().width;
        let mut max_w = bc.max().width;
        // toggle: h x h

        // place toggle top line
        let mut x = 0.0;

        //if show_toggle {
        // layout toggle button
        let toggle_pos = Point::ORIGIN;
        self.toggle.layout(
            ctx,
            &BoxConstraints::tight(Size::new(h, h)),
            &self.expanded,
            env,
        );
        self.toggle.set_origin(ctx, &self.expanded, env, toggle_pos);
        //}

        // offset position and reduce available width for the widget
        x += h;
        min_w = f64::max(min_w - h, 0.0);
        max_w = f64::max(max_w - h, 0.0);

        // place widget
        let widget_pos = Point::new(x, 0.0);
        let mut widget_bc = BoxConstraints::new(Size::new(min_w, 0.0), Size::new(max_w, h));
        let widget_size = self.widget.layout(ctx, &widget_bc, &data.node, env);
        self.widget.set_origin(ctx, &data.node, env, widget_pos);

        // place children below
        let mut y = h;
        let mut child_w = widget_size.width;
        if self.expanded {
            for (i, c) in self.children.iter_mut().enumerate() {
                let child_bc =
                    BoxConstraints::new(Size::new(min_w, 0.0), Size::new(max_w, bc.max().height));

                let child_data = data.child_data(i);
                let child_size = c.layout(ctx, &child_bc, &child_data, env);
                c.set_origin(ctx, &child_data, env, Point::new(h, y));
                y += child_size.height;
                child_w = child_w.max(child_size.width);
            }
        }

        bc.constrain(Size::new(x + child_w, y))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &TreeSelectionModel<T>, env: &Env) {
        let h = env.get(druid::theme::BASIC_WIDGET_HEIGHT);
        let half_h = 0.5 * h;

        if data.is_selected() {
            // selection background
            ctx.fill(self.widget.paint_rect(), &Color::rgb8(245, 194, 66));
        }

        self.widget.paint(ctx, &data.node, env);

        let has_children = data.node.children_count() != 0;
        if has_children {
            self.toggle.paint(ctx, &self.expanded, env);
            if self.expanded {
                let x_tree_line = h + half_h + 0.5;
                let x_tree_line_end = 2.0 * h + 0.5;
                let y_tree_line_start = h + 0.5;
                let y_tree_line_end = self.children.last().unwrap().layout_rect().y0 + half_h + 0.5;

                // vertical tree line
                ctx.stroke(
                    Line::new(
                        Point::new(x_tree_line, y_tree_line_start),
                        Point::new(x_tree_line, y_tree_line_end),
                    ),
                    &Color::grey(0.8),
                    1.0,
                );

                for (i, c) in self.children.iter_mut().enumerate() {
                    let child_y = c.layout_rect().y0 + half_h + 0.5;
                    // horizontal tree line
                    ctx.stroke(
                        Line::new(
                            Point::new(x_tree_line, child_y),
                            Point::new(x_tree_line_end, child_y),
                        ),
                        &Color::grey(0.8),
                        1.0,
                    );
                    c.paint(ctx, &data.child_data(i), env);
                }
            }
        }
    }
}

pub struct TreeView<T: TreeModel + Data, W> {
    root: WidgetPod<TreeSelectionModel<T>, TreeNodeWidget<T, W>>,
}

impl<T: TreeModel + Data, W: Widget<T> + 'static> TreeView<T, W> {
    pub fn new(closure: impl Fn() -> W + 'static) -> TreeView<T, W> {
        let closure = Arc::new(closure);
        let root_widget = (closure)();
        TreeView {
            root: WidgetPod::new(TreeNodeWidget::new(root_widget, closure)),
        }
    }
}

impl<T: TreeModel + Data, W: Widget<T> + 'static> Widget<TreeSelectionModel<T>> for TreeView<T, W> {
    fn event(
        &mut self,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut TreeSelectionModel<T>,
        env: &Env,
    ) {
        self.root.event(ctx, event, data, env)
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &TreeSelectionModel<T>,
        env: &Env,
    ) {
        self.root.lifecycle(ctx, event, data, env)
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &TreeSelectionModel<T>,
        data: &TreeSelectionModel<T>,
        env: &Env,
    ) {
        self.root.update(ctx, data, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &TreeSelectionModel<T>,
        env: &Env,
    ) -> Size {
        let size = self.root.layout(ctx, bc, data, env);
        self.root.set_origin(ctx, data, env, Point::ORIGIN);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &TreeSelectionModel<T>, env: &Env) {
        self.root.paint(ctx, data, env)
    }
}
