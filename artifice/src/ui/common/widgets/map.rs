use std::marker::PhantomData;
use crate::ui::common::ActionSink;
use crate::ui::common::BoxConstraints;
use crate::ui::common::Size;
use crate::ui::common::LayoutCtx;
use crate::ui::common::PaintCtx;
use crate::ui::common::View;
use crate::ui::common::EventCtx;
use crate::ui::common::view::ViewEvent;

use crate::util::model::Data;
use crate::util::model::Revision;

pub struct ActionTransformer<'a, A, B, F> {
    parent: &'a mut dyn ActionSink<B>,
    transform: &'a F,
    _phantom: PhantomData<*const A>,
}

impl<'a, A, B, F: Fn(A) -> B> ActionTransformer<'a, A, B, F> {
    pub fn new(
        parent: &'a mut dyn ActionSink<B>,
        transform: &'a F,
    ) -> ActionTransformer<'a, A, B, F> {
        ActionTransformer {
            parent,
            transform,
            _phantom: PhantomData,
        }
    }
}

impl<'a, A, B, F: Fn(A) -> B> ActionSink<A> for ActionTransformer<'a, A, B, F> {
    fn emit(&mut self, action: A) {
        self.parent.emit((&self.transform)(action));
    }
}

pub struct Map<V, F> {
    inner: V,
    f: F,
}

impl<S: Data, V: View<S>, A, F: Fn(V::Action) -> A + 'static> View<S> for Map<V, F> {
    type Action = A;

    fn event(&mut self, e: &ViewEvent, ctx: &mut EventCtx<A>) {
        let mut sink = ActionTransformer::new(ctx.action_sink(), &self.f);
        self.inner.event(e, &mut EventCtx::new(&mut sink))
    }

    fn update(&mut self, state: &Revision<S>) {
        self.inner.update(state)
    }

    fn paint(&mut self, state: &S, ctx: &mut PaintCtx) -> bool {
        self.inner.paint(state, ctx)
    }

    fn layout(&mut self, state: &S, ctx: &mut LayoutCtx, constraints: &BoxConstraints) -> Size {
        unimplemented!()
    }
}

impl<V, F> Map<V, F> {
    pub fn new(inner: V, f: F) -> Map<V, F> {
        Map { inner, f }
    }
}