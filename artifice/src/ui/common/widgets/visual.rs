//! All sizes in logical pixels.

use direct2d::render_target::RenderTarget;
use direct2d::render_target::IRenderTarget;
use crate::ui::common::widgets::CheckboxState;
use crate::ui::common::PaintCtx;

pub const CHECKBOX_SIZE_L: (u32,u32) = (12,12);

pub fn draw_checkbox(state: CheckboxState, ctx: &mut PaintCtx) {
    let target = ctx.direct2d_target();

    // TODO
}
