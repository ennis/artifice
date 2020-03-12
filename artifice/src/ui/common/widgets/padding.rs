use crate::ui::common::view::*;
use crate::ui::common::BoxConstraints;
use crate::util::model::Revision;
use euclid::{Vector2D, UnknownUnit};
use winit::event::WindowEvent;

pub struct Padding<V> {
    inner: V,
    padding: f64
}

impl<V,S> View<S> for Padding<V> where V: View<S> {
    type Action = V::Action;

    fn event(&mut self, e: &WindowEvent, a: &mut EventCtx<Self::Action>) {
        unimplemented!()
    }

    fn update(&mut self, s: &Revision<S>) {
        self.inner.update(s)
    }

    fn paint(&mut self, state: &S, ctx: &mut PaintCtx) -> bool {
        self.inner.paint(state, ctx)
    }

    fn layout(&mut self, state: &S, ctx: &mut LayoutCtx, constraints: &BoxConstraints) -> Vector2D<f64, UnknownUnit> {
        let rect = constraints.contract(2.0 * self.padding);

    }
}