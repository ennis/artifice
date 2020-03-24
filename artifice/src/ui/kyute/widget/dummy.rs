use crate::ui::kyute::layout::{PaintLayout, BoxConstraints, Layout, Size};
use crate::ui::kyute::renderer::{Painter, Renderer};
use crate::ui::kyute::visual::{Node, Visual};
use crate::ui::kyute::{Widget, Point};
use euclid::{Point2D, UnknownUnit};
use crate::ui::kyute::event::Event;

/// Dummy widget that does nothing.
pub struct DummyWidget;

impl<A: 'static> Widget<A> for DummyWidget {
    type Visual = DummyVisual;

    fn layout(self, _renderer: &Renderer, _constraints: &BoxConstraints) -> Node<DummyVisual> {
        Node::new(Layout::new(Size::new(0.0, 0.0)), DummyVisual)
    }
}

pub struct DummyVisual;

impl<A> Visual<A> for DummyVisual {
    fn paint(&mut self, _painter: &mut Painter, _layout: &PaintLayout) {}

    fn hit_test(&mut self, _point: Point, _layout: &PaintLayout) -> bool {
        unimplemented!()
    }

    fn event(&mut self, _event: &Event, _layout: &PaintLayout) -> Vec<A> {
        unimplemented!()
    }
}
