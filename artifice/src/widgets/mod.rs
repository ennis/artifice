//! Various useful widgets
pub mod tree;

use druid::{
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Size,
    UpdateCtx, Widget, WidgetPod, Data
};

// TODO:
// - a proper "stepper" widget
// - multi-dimensional data editors (vec2/vec3/vec4)
// - button boxes
// - form layout
// - 3D views


/// Rebuilds a widget with the provided closure whenever the data changes.
pub struct RebuildOnDataChange<T, W, F> {
    closure: F,
    inner: Option<WidgetPod<T, W>>,
}

impl<W, T, F> RebuildOnDataChange<T, W, F>
    where
        F: Fn(&T) -> W,
{
    pub fn new(closure: F) -> RebuildOnDataChange<T, W, F> {
        RebuildOnDataChange {
            closure,
            inner: None,
        }
    }
}

impl<W, T, F> Widget<T> for RebuildOnDataChange<T, W, F>
    where
        W: Widget<T>,
        T: Data,
        F: Fn(&T) -> W,
{
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Some(ref mut inner) = self.inner {
            inner.event(ctx, event, data, env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if self.inner.is_none() {
            let mut inner = WidgetPod::new((self.closure)(data));
            self.inner = Some(inner);
            ctx.children_changed();
        }

        self.inner
            .as_mut()
            .unwrap()
            .lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        if !old_data.same(data) {
            self.inner.replace(WidgetPod::new((self.closure)(data)));
            ctx.children_changed();
        }

        self.inner.as_mut().unwrap().update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        if let Some(ref mut inner) = self.inner {
            inner.layout(ctx, bc, data, env)
        } else {
            Size::ZERO
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if let Some(ref mut inner) = self.inner {
            inner.paint(ctx, data, env);
        }
    }
}
