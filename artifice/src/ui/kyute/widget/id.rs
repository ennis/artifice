use crate::ui::kyute::layout::BoxConstraints;
use crate::ui::kyute::renderer::Renderer;
use crate::ui::kyute::visual::Node;
use crate::ui::kyute::Widget;
use std::hash::Hash;

/// Identifies a widget.
pub struct Id<W> {
    inner: W,
}

impl<W> Id<W> {
    pub fn new(_id: impl Hash, inner: W) -> Id<W> {
        Id { inner }
    }
}

impl<A: 'static, W: Widget<A>> Widget<A> for Id<W> {
    type Visual = W::Visual;

    fn layout(self, renderer: &Renderer, constraints: &BoxConstraints) -> Node<W::Visual> {
        self.inner.layout(renderer, constraints)
    }
}
