//! Popup windows (for context menus and such)
use crate::ui::common::platform::PlatformWindow;
use crate::ui::common::WindowEventTarget;
use crate::ui::common::{EventResult, WindowCtx};
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::window::{WindowBuilder, WindowId};

pub struct ContextMenu {
    window: PlatformWindow,
}

impl ContextMenu {
    pub fn open(ctx: &mut WindowCtx) {
        let builder = WindowBuilder::new()
            .with_decorations(false)
            .with_inner_size(PhysicalSize::new(100, 300));
        let window =
            PlatformWindow::new(ctx, builder, false).expect("failed to create platform window");
        let context_menu = ContextMenu { window };

        ctx.add_window(context_menu);
    }
}

impl WindowEventTarget for ContextMenu {
    fn window_id(&self) -> WindowId {
        self.window.window().id()
    }

    fn event(&mut self, _ctx: &mut WindowCtx, event: WindowEvent) -> EventResult {
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

    fn paint(&mut self, _ctx: &mut WindowCtx) {}
}
