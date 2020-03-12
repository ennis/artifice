use std::marker::PhantomData;
use crate::util::model::Lens;
use crate::util::model::Data;
use crate::util::model::Revision;
use crate::ui::common::PaintCtx;
use crate::ui::common::Size;
use crate::ui::common::View;
use crate::ui::common::LayoutCtx;
use crate::ui::common::EventCtx;
use crate::ui::common::BoxConstraints;
use crate::ui::common::ViewEvent;
use winit::event::Event;

pub struct Button<S: Data, Label: Lens<S, String>> {
    label: Label,
    _phantom: PhantomData<S>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ButtonAction {
    Clicked,
    Released,
}

impl<S: Data, Label: Lens<S, String>> Button<S, Label> {
    pub fn new(label: Label) -> Button<S, Label> {
        Button {
            label,
            _phantom: PhantomData,
        }
    }
}

impl<S: Data, Label: Lens<S, String>> View<S> for Button<S, Label> {
    type Action = ButtonAction;

    fn event(&mut self, _e: &ViewEvent, _a: &mut EventCtx<Self::Action>) {
        unimplemented!()
    }

    fn update(&mut self, _state: &Revision<S>) {
        unimplemented!()
    }

    fn paint(&mut self, _state: &S, _ctx: &mut PaintCtx) -> bool {
        false
    }

    fn layout(&mut self, state: &S, ctx: &mut LayoutCtx, constraints: &BoxConstraints) -> Size {
        unimplemented!()
    }
}
