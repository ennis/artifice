use std::marker::PhantomData;
use crate::util::model::Data;
use crate::util::model::Revision;
use crate::ui::common::view::{View, LayoutCtx};
use crate::ui::common::view::ViewEvent;
use crate::ui::common::view::EventCtx;
use crate::ui::common::layout::{BoxConstraints, Size};
use crate::ui::common::PaintCtx;

pub struct Label<A> {
    text: String,
    _phantom: PhantomData<*const A>,
}

impl<A> Label<A> {
    pub fn new() -> Self {
        Label {
            text: "".into(),
            _phantom: PhantomData,
        }
    }
}

impl<S: Data, A> View<S> for Label<A> {
    type Action = A;

    fn event(&mut self, _e: &ViewEvent, _ctx: &mut EventCtx<A>) {
        unimplemented!()
    }

    fn update(&mut self, _rev: &Revision<S>) {}

    fn paint(&mut self, _state: &S, _ctx: &mut PaintCtx) -> bool {
        unimplemented!()
    }

    fn layout(&mut self, state: &S, ctx: &mut LayoutCtx, constraints: &BoxConstraints) -> Size {
        unimplemented!()
    }
}