use crate::{
    event::PointerEventKind,
    style::{BoxStyle, ColorRef, ValueRef},
    theme,
    widget::{prelude::*, Container, Label},
    SideOffsets, Signal,
};
use tracing::trace;

/// Button styling information.
#[derive(Clone, Debug, Default)]
pub struct ButtonStyle {
    pub box_style: BoxStyle,
    pub label_color: ColorRef,
}

impl_env_value!(ButtonStyle);

#[derive(Clone)]
pub struct Button {
    inner: Container<Label>,
    clicked: Signal<()>,
}

impl Button {
    /// Creates a new button with the specified label.
    #[composable]
    pub fn new(label: String) -> Button {
        Button {
            inner: Container::new(#[compose] Label::new(label))
                .min_height(theme::BUTTON_HEIGHT)
                .content_padding(SideOffsets::new_all_same(5.0))
                .baseline(theme::BUTTON_LABEL_BASELINE)
                .box_style(theme::BUTTON),
            clicked: #[compose] Signal::new(),
        }
    }

    /// Sets the style of this button.
    pub fn box_style(mut self, style: impl Into<ValueRef<BoxStyle>>) -> Button {
        self.set_box_style(style);
        self
    }

    /// Sets the style of this button.
    pub fn set_box_style(&mut self, style: impl Into<ValueRef<BoxStyle>>) {
        self.inner.set_box_style(style.into());
    }

    /// Sets the text color of this button.
    pub fn text_color(mut self, color: impl Into<ColorRef>) -> Button {
        self.set_text_color(color);
        self
    }

    /// Sets the text color of this button.
    pub fn set_text_color(&mut self, color: impl Into<ColorRef>) {
        self.inner.contents_mut().set_color(color);
    }

    /// Returns whether this button has been clicked.
    #[composable]
    pub fn clicked(&self) -> bool {
        #[compose]
        self.clicked.signalled()
    }
}

impl Widget for Button {
    fn debug_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn event(&self, ctx: &mut EventCtx, event: &mut Event, env: &Environment) {
        match event {
            Event::Pointer(p) => match p.kind {
                PointerEventKind::PointerDown => {
                    //trace!("button clicked");
                    self.clicked.signal(ctx, ());
                    ctx.request_focus();
                    ctx.request_redraw();
                    ctx.set_handled();
                }
                PointerEventKind::PointerOver => {
                    //trace!("button PointerOver");
                    ctx.request_redraw();
                }
                PointerEventKind::PointerOut => {
                    //trace!("button PointerOut");
                    ctx.request_redraw();
                }
                _ => {}
            },
            _ => {}
        }

        if !ctx.handled() {
            self.inner.event(ctx, event, env)
        }
    }

    fn layout(
        &self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        env: &Environment,
    ) -> Measurements {
        let m = self.inner.layout(ctx, constraints, env);
        m
    }

    fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, env: &Environment) {
        self.inner.paint(ctx, bounds, env)
    }
}
