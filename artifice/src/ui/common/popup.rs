//! Popup windows (for context menus and such)
use crate::ui::common::view::View;
use crate::ui::common::view::EventCtx;
use crate::ui::common::view::ViewEvent;
use crate::ui::common::view::LayoutCtx;
use crate::util::model::Revision;
use crate::util::model::Data;
use crate::ui::common::layout::{BoxConstraints, Size};
use crate::ui::common::PaintCtx;
use crate::ui::common::platform::PlatformWindow;
use crate::ui::common::{WindowCtx, EventResult};
use crate::ui::common::WindowEventTarget;
use winit::window::{WindowBuilder, WindowId};
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;

#[derive(Clone,Data)]
pub struct ContextMenuItem {
    pub title: String,
}

struct ContextMenuView {

}

impl View<()> for ContextMenuView {
    /// Selected item.
    type Action = usize;

    fn event(&mut self, e: &ViewEvent, a: &mut EventCtx<Self::Action>) {

    }

    fn update(&mut self, _s: &Revision<()>) {

    }

    fn paint(&mut self, state: &(), ctx: &mut PaintCtx) -> bool {
        unimplemented!()
    }

    fn layout(&mut self, state: &(), ctx: &mut LayoutCtx, constraints: &BoxConstraints) -> Size {
        unimplemented!()
    }
}


pub struct ContextMenu {
    window: PlatformWindow,
    view: ContextMenuView,
}

impl ContextMenu {
    pub fn open(ctx: &mut WindowCtx)
    {
        let builder = WindowBuilder::new().with_decorations(false).with_inner_size(PhysicalSize::new(100, 300));
        let window = PlatformWindow::new(ctx, builder, false).expect("failed to create platform window");
        let context_menu = ContextMenu {
            window,
            view: ContextMenuView {}
        };
        ctx.add_window(context_menu);
    }
}

impl WindowEventTarget for ContextMenu {
    fn window_id(&self) -> WindowId {
        self.w.window().id()
    }

    fn event(&mut self, ctx: &mut WindowCtx, event: WindowEvent) -> EventResult {
        match event {
            WindowEvent::Focused(false) => {
                // lost focus, so close the window
                EventResult::Close
            }
            _ => {
                // handle the rest
                EventResult::None
            }
        }
    }

    fn paint(&mut self, ctx: &mut WindowCtx) {
        {
            let d2d = self.w.draw_2d();
            let target = d2d.target;
            // TODO draw stuff
        }
        self.w.present();
    }
}

