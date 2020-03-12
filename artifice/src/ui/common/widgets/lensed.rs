use std::marker::PhantomData;
use crate::ui::common::ViewEvent;
use crate::ui::common::View;
use crate::ui::common::EventCtx;
use crate::ui::common::BoxConstraints;
use crate::ui::common::Size;
use crate::ui::common::LayoutCtx;
use crate::ui::common::PaintCtx;
use crate::util::model::Data;
use crate::util::model::Revision;
use crate::util::model::Lens;

pub struct Lensed<B, L, V> {
    lens: L,
    inner: V,
    _phantom: PhantomData<B>,
}

impl<A, B, L, V> View<A> for Lensed<B, L, V>
where
    A: Data,
    B: Data,
    L: Lens<A, B>,
    V: View<B>,
{
    type Action = V::Action;

    fn event(&mut self, e: &ViewEvent, a: &mut EventCtx<Self::Action>) {
        self.inner.event(e, a)
    }

    fn update(&mut self, state: &Revision<A>) {
        let inner = &mut self.inner;
        self.lens.focus(state, |state| inner.update(state));
    }

    fn paint(&mut self, state: &A, ctx: &mut PaintCtx) -> bool {
        let inner = &mut self.inner;
        self.lens.with(state, |state| inner.paint(state, ctx))
    }

    fn layout(&mut self, state: &A, ctx: &mut LayoutCtx, constraints: &BoxConstraints) -> Size {
        unimplemented!()
    }
}

impl<B, L, V> Lensed<B, L, V> {
    pub fn new(lens: L, inner: V) -> Lensed<B, L, V> {
        Lensed {
            lens,
            inner,
            _phantom: PhantomData,
        }
    }
}