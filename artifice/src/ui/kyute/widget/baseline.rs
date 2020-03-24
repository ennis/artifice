use crate::ui::kyute::layout::{BoxConstraints, Layout, Offset, Size};
use crate::ui::kyute::renderer::{Renderer};
use crate::ui::kyute::visual::Node;
use crate::ui::kyute::Widget;

/// .
pub struct Baseline<W> {
    inner: W,
    baseline: f64,
}

impl<W> Baseline<W> {
    pub fn new(baseline: f64, inner: W) -> Baseline<W> {
        Baseline { inner, baseline }
    }
}

impl<A: 'static, W: Widget<A>> Widget<A> for Baseline<W> {
    type Visual = Node<W::Visual>;

    fn layout(self, renderer: &Renderer, constraints: &BoxConstraints) -> Node<Self::Visual> {
        let mut child = self.inner.layout(renderer, constraints);
        let off = self.baseline - child.layout.baseline.unwrap_or(child.layout.size.height);
        let height = child.layout.size.height + off;
        child.layout.offset.y = off;

        let layout = Layout {
            offset: Offset::new(0.0, 0.0),
            size: constraints.constrain(Size::new(child.layout.size.width, height)),
            baseline: Some(self.baseline),
        };

        Node::new(layout, child)
    }
}
