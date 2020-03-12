use std::marker::PhantomData;

use crate::ui::common::LayoutCtx;
use crate::ui::common::View;
use crate::ui::common::EventCtx;
use crate::ui::common::ViewEvent;
use crate::ui::common::PaintCtx;
use crate::ui::common::BoxConstraints;
use crate::ui::common::Size;

use crate::util::model::LensExt;
use crate::util::model::Revision;
use crate::util::model::Lens;
use crate::util::model::Data;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum CheckboxState {
    Unchecked,
    PartiallyChecked,
    Checked,
}

pub struct Checkbox<S: Data, Label: Lens<S, String>, State: Lens<S, CheckboxState>> {
    label: Label,
    state: State,
    _phantom: PhantomData<S>,
}

impl<S: Data, Label: Lens<S, String>, State: Lens<S, CheckboxState>> Checkbox<S, Label, State> {
    pub fn new(label: Label, state: State) -> Self {
        Checkbox {
            label,
            state,
            _phantom: PhantomData,
        }
    }
}

impl<S: Data, Label: Lens<S, String>, State: Lens<S, CheckboxState>> View<S>
    for Checkbox<S, Label, State>
{
    type Action = CheckboxState;

    fn event(&mut self, _e: &ViewEvent, _a: &mut EventCtx<CheckboxState>) {
        unimplemented!()
    }

    fn update(&mut self, _s: &Revision<S>) {}

    fn paint(&mut self, s: &S, ctx: &mut PaintCtx) -> bool {
        let checked = self.state.get(s);
        draw_checkbox(checked, "test", ctx);
        false
    }

    fn layout(&mut self, state: &S, ctx: &mut LayoutCtx, constraints: &BoxConstraints) -> Size {
        unimplemented!()
    }
}


fn draw_checkbox(_state: CheckboxState, _label: &str, ctx: &mut PaintCtx)
{
}