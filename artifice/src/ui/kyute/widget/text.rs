use crate::ui::kyute::layout::{PaintLayout, BoxConstraints, Layout, Size};
use crate::ui::kyute::renderer::{Painter, Renderer, TextLayout};
use crate::ui::kyute::visual::{Node, Visual};
use crate::ui::kyute::{Widget, Point};
use euclid::{Point2D, UnknownUnit};
use crate::ui::kyute::event::Event;

pub struct TextVisual {
    text: String,
    text_layout: TextLayout,
}

impl<A> Visual<A> for TextVisual {
    fn paint(&mut self, painter: &mut Painter, layout: &PaintLayout) {
        painter.draw_text(layout.bounds.origin, &self.text_layout)
    }

    fn hit_test(&mut self, point: Point, layout: &PaintLayout) -> bool {
        unimplemented!()
    }

    fn event(&mut self, event: &Event, layout: &PaintLayout) -> Vec<A> {
        unimplemented!()
    }
}

/// Text element.
pub struct Text {
    text: String,
}

impl<A: 'static> Widget<A> for Text {
    type Visual = TextVisual;

    fn layout(self, renderer: &Renderer, constraints: &BoxConstraints) -> Node<TextVisual> {
        let text = self.text;

        let text_layout = renderer.layout_text(&text, constraints.biggest());
        let text_size = Size::new(
            text_layout.metrics().width as f64,
            text_layout.metrics().height as f64,
        )
        .ceil();

        let baseline = text_layout
            .line_metrics()
            .first()
            .map(|m| m.baseline.ceil() as f64);

        let layout = Layout::new(text_size).with_baseline(baseline);
        Node::new(layout, TextVisual { text, text_layout })
    }
}

impl Text {
    pub fn new(text: impl Into<String>) -> Text {
        Text { text: text.into() }
    }
}
