use crate::{
    composable,
    state::State,
    widget::{separator::separator, Clickable, Container, Flex, Grid, GridLength, Image, Label, SingleChildWidget},
    Color, Orientation, SideOffsets, Signal, Widget, WidgetExt,
};

/// A widget with a title.
#[derive(Clone)]
pub struct TitledPane {
    inner: Flex,
    collapsed_changed: Option<bool>,
}

impl TitledPane {
    /// Creates a new collapsible pane.
    #[composable]
    pub fn collapsible(
        title: impl Into<String>,
        initially_collapsed: bool,
        content: impl Widget + 'static,
    ) -> TitledPane {
        let collapsed_state = State::new(|| initially_collapsed);
        let pane = Self::new(collapsed_state.get(), title.into(), content);
        collapsed_state.update(pane.collapsed_changed());
        pane
    }

    #[composable]
    fn new(collapsed: bool, title: String, content: impl Widget + 'static) -> TitledPane {
        let mut inner = Flex::new(Orientation::Vertical);

        use kyute::style::*;

        let icon = Image::from_uri(if collapsed {
            "data/icons/chevron-collapsed.png"
        } else {
            "data/icons/chevron.png"
        });

        // Title bar
        let title_bar = Clickable::new(
            Container::new(
                Grid::with_columns([GridLength::Fixed(20.0), GridLength::Fixed(3.0), GridLength::Flex(1.0)])
                    .with(0, 0, icon)
                    .with(0, 2, Label::new(title).centered()),
            )
            .content_padding(SideOffsets::new_all_same(2.0))
            .box_style(BoxStyle::new().fill(Color::from_hex("#111111"))),
        );

        let collapsed_changed = if title_bar.clicked() { Some(!collapsed) } else { None };

        inner.push(title_bar);
        inner.push(separator(Orientation::Horizontal));

        // Add contents if not collapsed
        if !collapsed {
            inner.push(content);
        }

        TitledPane {
            inner,
            collapsed_changed,
        }
    }

    /// Returns whether the panel has been collapsed or expanded from user input.
    pub fn collapsed_changed(&self) -> Option<bool> {
        self.collapsed_changed
    }
}

impl SingleChildWidget for TitledPane {
    fn child(&self) -> &dyn Widget {
        &self.inner
    }
}
