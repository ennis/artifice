//! Tree view widget
use druid::{
    kurbo::Line, widget::Button, BoxConstraints, Color, Data, Env, Event, EventCtx, LayoutCtx,
    Lens, LifeCycle, LifeCycleCtx, PaintCtx, Point, RenderContext, Size, UpdateCtx,
    Widget, WidgetPod,
};
use std::sync::Arc;

/// Model for a tree node.
pub trait TreeNodeModel: Data {
    /// Returns the number of child nodes.
    fn child_count(&self) -> usize;

    /// Runs the provided closure with a reference to the specified child node.
    fn with_child<V, F: FnOnce(&Self) -> V>(&self, index: usize, f: F) -> V;

    /// Runs the provided closure with a mutable reference to the specified child node.
    fn with_child_mut<V, F: FnOnce(&mut Self) -> V>(&mut self, index: usize, f: F) -> V;
}

/// Combination of a node and a list of selected nodes.
#[derive(Clone, Data, Debug)]
pub struct TreeNodeData<T> {
    pub node: T,
    pub selection: Arc<Vec<T>>,
}

impl<T> TreeNodeData<T>
where
    T: TreeNodeModel,
{
    /// Creates a new TreeNodeData with an empty selection.
    pub fn new(root: T) -> TreeNodeData<T> {
        TreeNodeData {
            node: root,
            selection: Arc::new(Vec::new()),
        }
    }

    /// Returns a lens over the `node` field.
    pub fn node_lens() -> impl Lens<Self, T> {
        druid::lens!(Self, node)
    }

    /// Returns a lens over the `selection` field.
    pub fn selection_lens() -> impl Lens<Self, Arc<Vec<T>>> {
        druid::lens!(Self, selection)
    }

    /// Returns whether `self.node` is selected (i.e. `self.selection` contains `self.node`).
    pub fn is_selected(&self) -> bool {
        self.selection
            .iter()
            .position(|x| x.same(&self.node))
            .is_some()
    }

    /// Runs the specified closure with the `TreeNodeData` for the child node at the specified index.
    // TODO figure out how to factor this out into a lens
    pub fn with_child_data<V, F: FnOnce(&Self) -> V>(&self, i: usize, f: F) -> V {
        let child_node = self.node.with_child(i, |n| n.clone());
        let child_data = TreeNodeData {
            node: child_node,
            selection: self.selection.clone(),
        };

        f(&child_data)
    }

    /// Runs the specified closure with the `TreeNodeData` for the child node at the specified index.
    pub fn with_child_data_mut<V, F: FnOnce(&mut Self) -> V>(&mut self, i: usize, f: F) -> V {
        let child_node = self.node.with_child(i, |n| n.clone());
        let mut child_data = TreeNodeData {
            node: child_node,
            selection: self.selection.clone(),
        };

        let result = f(&mut child_data);

        if !self.selection.same(&child_data.selection) {
            self.selection = child_data.selection.clone();
        }

        self.node.with_child_mut(i, |n| {
            if !n.same(&child_data.node) {
                *n = child_data.node.clone();
            }
        });

        result
    }
}

/// A widget that represents a node in a tree data structure.
pub struct TreeNodeWidget<T, W> {
    expanded: bool,
    toggle: WidgetPod<bool, Box<dyn Widget<bool>>>,
    widget: WidgetPod<TreeNodeData<T>, W>,
    children: Vec<WidgetPod<TreeNodeData<T>, Self>>,
    /// Creates child widgets
    closure: Arc<dyn Fn() -> W>,
}

impl<T, W> TreeNodeWidget<T, W>
where
    T: TreeNodeModel,
    W: Widget<TreeNodeData<T>>,
{
    pub fn new(widget: W, closure: Arc<dyn Fn() -> W>) -> TreeNodeWidget<T, W> {
        let toggle = Button::new("").on_click(|ctx: &mut EventCtx, data: &mut bool, env: &_| {
            *data = !*data;
            ctx.set_handled();
        });

        TreeNodeWidget {
            expanded: false,
            toggle: WidgetPod::new(Box::new(toggle)),
            widget: WidgetPod::new(widget),
            children: vec![],
            closure,
        }
    }

    fn create_children(&mut self, data: &T) {
        for _ in 0..data.child_count() {
            let child = TreeNodeWidget::new((self.closure)(), self.closure.clone());
            self.children.push(WidgetPod::new(child));
        }
    }
}

impl<T, W> Widget<TreeNodeData<T>> for TreeNodeWidget<T, W>
where
    T: TreeNodeModel,
    W: Widget<TreeNodeData<T>>,
{
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut TreeNodeData<T>, env: &Env) {
        let prev_expanded = self.expanded;
        self.toggle.event(ctx, event, &mut self.expanded, env);
        for (i, c) in self.children.iter_mut().enumerate() {
            data.with_child_data_mut(i, |child_data| {
                c.event(ctx, event, child_data, env);
            });
        }

        // expanding
        if prev_expanded != self.expanded {
            if self.expanded {
                // may be expanding for the first time
                if self.children.is_empty() {
                    // create widgets for the nodes
                    self.create_children(&data.node);
                    ctx.children_changed();
                }
            }

            ctx.request_layout();
        }

        if !ctx.is_handled() {
            if let Event::MouseUp(mouse_event) = event {
                if mouse_event.mods.ctrl() {
                    // toggle selection
                    if data.is_selected() {
                        eprintln!("removing from selection");
                        if let Some(index) = data.selection.iter().position(|x| x.same(&data.node))
                        {
                            Arc::make_mut(&mut data.selection).swap_remove(index);
                        }
                    } else {
                        eprintln!("adding to selection");
                        Arc::make_mut(&mut data.selection).push(data.node.clone());
                    }
                } else if mouse_event.mods.shift() {
                    // TODO add range
                } else {
                    // set selection
                    let selection = Arc::make_mut(&mut data.selection);
                    selection.clear();
                    selection.push(data.node.clone());
                }

                // toggle selection
                ctx.request_paint();
                ctx.set_handled();
            }
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &TreeNodeData<T>,
        env: &Env,
    ) {
        self.toggle.lifecycle(ctx, event, &self.expanded, env);
        self.widget.lifecycle(ctx, event, &data, env);
        for (i, c) in self.children.iter_mut().enumerate() {
            data.with_child_data(i, |child_data| {
                c.lifecycle(ctx, event, child_data, env);
            });
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &TreeNodeData<T>,
        data: &TreeNodeData<T>,
        _env: &Env,
    ) {
        if !old_data.selection.same(&data.selection) {
            ctx.request_paint();
        }

        if !old_data.node.same(&data.node) {
            // we could do a diff, but for now just rebuild all children
            // TODO it's important to do a precise diff because otherwise we lose the state of the "expanded" flag
            self.children.clear();
            self.create_children(&data.node);
            ctx.children_changed();
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &TreeNodeData<T>,
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
        let toggle_pos = Point::new(2.0, 2.0);
        self.toggle.layout(
            ctx,
            &BoxConstraints::tight(Size::new(h - 4.0, h - 4.0)),
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
        let widget_size = self.widget.layout(ctx, &widget_bc, &data, env);
        self.widget.set_origin(ctx, &data, env, widget_pos);

        // place children below
        let mut y = h;
        let mut child_w = widget_size.width;
        if self.expanded {
            for (i, c) in self.children.iter_mut().enumerate() {
                let child_bc =
                    BoxConstraints::new(Size::new(min_w, 0.0), Size::new(max_w, bc.max().height));

                let child_size = data.with_child_data(i, |data| {
                    let size = c.layout(ctx, &child_bc, data, env);
                    c.set_origin(ctx, data, env, Point::new(h, y));
                    size
                });

                y += child_size.height;
                child_w = child_w.max(child_size.width);
            }
        }

        bc.constrain(Size::new(x + child_w, y))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &TreeNodeData<T>, env: &Env) {
        let h = env.get(druid::theme::BASIC_WIDGET_HEIGHT);
        let half_h = 0.5 * h;

        if data.is_selected() {
            // selection background
            ctx.fill(self.widget.paint_rect(), &Color::rgb8(245, 194, 66));
        }

        self.widget.paint(ctx, &data, env);

        let has_children = data.node.child_count() != 0;
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
                    data.with_child_data(i, |data| {
                        c.paint(ctx, data, env);
                    });
                }
            }
        }
    }
}

pub struct TreeView<T, W> {
    root: WidgetPod<TreeNodeData<T>, TreeNodeWidget<T, W>>,
}

impl<T, W> TreeView<T, W>
where
    T: TreeNodeModel,
    W: Widget<TreeNodeData<T>>,
{
    pub fn new(closure: impl Fn() -> W + 'static) -> TreeView<T, W> {
        let closure = Arc::new(closure);
        let root_widget = (closure)();
        TreeView {
            root: WidgetPod::new(TreeNodeWidget::new(root_widget, closure)),
        }
    }
}

impl<T, W> Widget<TreeNodeData<T>> for TreeView<T, W>
where
    T: TreeNodeModel,
    W: Widget<TreeNodeData<T>>,
{
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut TreeNodeData<T>, env: &Env) {
        self.root.event(ctx, event, data, env)
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &TreeNodeData<T>,
        env: &Env,
    ) {
        self.root.lifecycle(ctx, event, data, env)
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &TreeNodeData<T>,
        data: &TreeNodeData<T>,
        env: &Env,
    ) {
        self.root.update(ctx, data, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &TreeNodeData<T>,
        env: &Env,
    ) -> Size {
        let size = self.root.layout(ctx, bc, data, env);
        self.root.set_origin(ctx, data, env, Point::ORIGIN);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &TreeNodeData<T>, env: &Env) {
        self.root.paint(ctx, data, env)
    }
}
