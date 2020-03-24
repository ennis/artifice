use crate::ui::kyute::layout::BoxConstraints;
use crate::ui::kyute::renderer::Renderer;
use crate::ui::kyute::visual::Node;
use crate::ui::kyute::Widget;

/// Expands the child widget to fill all its available space.
pub struct Expand<W>(pub W);

impl<A: 'static, W> Widget<A> for Expand<W>
where
    W: Widget<A>,
{
    type Visual = W::Visual;

    fn layout(self, renderer: &Renderer, constraints: &BoxConstraints) -> Node<W::Visual> {
        self.0.layout(renderer, &BoxConstraints::tight(constraints.biggest()))
    }
}
