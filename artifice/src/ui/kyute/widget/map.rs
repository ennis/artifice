use crate::ui::kyute::layout::{PaintLayout, BoxConstraints};
use crate::ui::kyute::renderer::{Painter, Renderer};
use crate::ui::kyute::visual::{Node, Visual};
use crate::ui::kyute::{Widget, Point};
use std::marker::PhantomData;
use euclid::{Point2D, UnknownUnit};
use crate::ui::kyute::event::Event;

pub struct MapVisual<A, V, F> {
    inner: V,
    map: F,
    _phantom: PhantomData<A>,
}

impl<A, B, V, F> Visual<B> for MapVisual<A, V, F>
where
    V: Visual<A>,
    F: Fn(A) -> B,
{
    fn paint(&mut self, painter: &mut Painter, layout: &PaintLayout) {
        self.inner.paint(painter, layout)
    }

    fn hit_test(&mut self, point: Point, layout: &PaintLayout) -> bool {
        self.inner.hit_test(point, layout)
    }

    fn event(&mut self, event: &Event, layout: &PaintLayout) -> Vec<B> {
        self.inner.event(event, layout).drain(..).map(|a| (self.map)(a)).collect()
    }
}

/// Map one action to another.
pub struct Map<A, W, F> {
    inner: W,
    map: F,
    _phantom: PhantomData<A>,
}

impl<A, W, F> Map<A, W, F> {
    pub fn new(inner: W, map: F) -> Map<A, W, F> {
        Map {
            inner,
            map,
            _phantom: PhantomData,
        }
    }
}

impl<A: 'static, B: 'static, W: Widget<A>, F: Fn(A) -> B + 'static> Widget<B> for Map<A, W, F> {
    type Visual = MapVisual<A, W::Visual, F>;

    fn layout(self, renderer: &Renderer, constraints: &BoxConstraints) -> Node<Self::Visual> {
        let node = self.inner.layout(renderer, constraints);

        Node::new(
            node.layout,
            MapVisual {
                inner: node.visual,
                map: self.map,
                _phantom: PhantomData,
            },
        )
    }
}
