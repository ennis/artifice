//! Elements of the visual tree (after layout): `Visual`s and `Node`s.
use crate::ui::kyute::layout::{PaintLayout, Point, Layout};
use crate::ui::kyute::renderer::Painter;
use crate::ui::kyute::renderer::Renderer;
use crate::ui::kyute::event::{Event, EventCtx};
use euclid::{Point2D, UnknownUnit};

/// The interface for painting a visual element on the screen, and handling events that target this
/// visual.
///
/// [`Visual`]s are typically wrapped in a [`Node`], which bundles the visual and the layout
/// information of the visual within a parent object.
pub trait Visual<A> {
    /// Draws the visual using the specified painter. `layout` specifies where on the screen the
    /// visual should be drawn.
    fn paint(&mut self, painter: &mut Painter, layout: &PaintLayout);

    /// Checks if the given point falls inside the widget.
    fn hit_test(&mut self, point: Point, layout: &PaintLayout) -> bool;

    /// Handles an event that targets this visual, and returns the _actions_ emitted in response
    /// to this event.
    fn event(&mut self, event_ctx: &EventCtx, event: &Event) -> Vec<A>;
}

/// Boxed visuals implementation.
impl<A> Visual<A> for Box<dyn Visual<A>> {
    fn paint(&mut self, painter: &mut Painter, layout: &PaintLayout) {
        self.as_mut().paint(painter, layout)
    }

    fn hit_test(&mut self, point: Point, layout: &PaintLayout) -> bool {
        self.as_mut().hit_test(point,  layout)
    }

    fn event(&mut self, event: &Event, layout: &PaintLayout) -> Vec<A> {
        self.as_mut().event(event, layout)
    }
}

/// Bundle of a [`Visual`] and a [`Layout`] that contains layout information about the visual in
/// a parent object.
///
/// [`Layout`]: super::layout::Layout
pub struct Node<V> {
    /// Layout data of the node
    pub layout: Layout,
    /// Visual
    pub visual: V,
}

impl<V> Node<V> {
    /// Creates a new node from a layout and a visual.
    pub fn new(layout: Layout, visual: V) -> Node<V> {
        Node { layout, visual }
    }

    /// Returns the same node but box the contained visual.
    pub fn boxed<A>(self) -> Node<Box<dyn Visual<A>>>
    where
        V: Visual<A> + 'static,
    {
        Node {
            layout: self.layout,
            visual: Box::new(self.visual),
        }
    }
}

/// Nodes can also be directly used as Visuals: they apply their layout's offset
/// to the `PaintLayout` before calling the wrapped visual.
impl<A, V: Visual<A>> Visual<A> for Node<V> {
    fn paint(&mut self, painter: &mut Painter, layout: &PaintLayout) {
        let layout = PaintLayout::new(layout.bounds.origin, &self.layout);
        self.visual.paint(painter, &layout)
    }

    fn hit_test(&mut self, point: Point, layout: &PaintLayout) -> bool {
        let layout = PaintLayout::new(layout.bounds.origin, &self.layout);
        self.visual.hit_test(point, &layout)
    }

    fn event(&mut self, event: &Event, layout: &PaintLayout) -> Vec<A> {
        let layout = PaintLayout::new(layout.bounds.origin, &self.layout);
        self.visual.event(event, &layout)
    }
}
