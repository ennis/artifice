use crate::ui::kyute::layout::PaintLayout;
use crate::ui::kyute::layout::Alignment;
use crate::ui::kyute::layout::BoxConstraints;
use crate::ui::kyute::layout::EdgeInsets;
use crate::ui::kyute::layout::Layout;
use crate::ui::kyute::layout::Size;
use crate::ui::kyute::renderer::ButtonState;
use crate::ui::kyute::renderer::Painter;
use crate::ui::kyute::renderer::Renderer;
use crate::ui::kyute::visual::Node;
use crate::ui::kyute::visual::Visual;
use crate::ui::kyute::widget::BoxedWidget;
use crate::ui::kyute::widget::Text;
use crate::ui::kyute::widget::Widget;
use crate::ui::kyute::widget::WidgetExt;
use euclid::{Point2D, UnknownUnit};
use crate::ui::kyute::event::Event;
use crate::ui::kyute::Point;

/// Button element.
pub struct Button<A> {
    label: BoxedWidget<A>,
    /// Action to emit on button click.
    on_click: Option<A>,
}

impl<A: 'static> Widget<A> for Button<A> {
    type Visual = ButtonVisual<A>;

    fn layout(self, renderer: &Renderer, constraints: &BoxConstraints) -> Node<ButtonVisual<A>> {
        let button_metrics = &renderer.widget_metrics().button_metrics;

        let mut label_visual = self.label.layout(
            renderer,
            &constraints.deflate(&EdgeInsets::all(button_metrics.label_padding.into())),
        );
        let button_size = Size::new(
            label_visual.layout.size.width + 2.0 * button_metrics.label_padding,
            label_visual.layout.size.height + 2.0 * button_metrics.label_padding,
        );
        let button_size = button_size.max(Size::new(
            button_metrics.min_width,
            button_metrics.min_height,
        ));

        let button_size = constraints.constrain(button_size);

        let mut layout = Layout::new(button_size);
        Layout::align(&mut layout, &mut label_visual.layout, Alignment::CENTER);

        Node::new(
            layout,
            ButtonVisual {
                label: label_visual,
                on_click: self.on_click,
            },
        )
    }
}

impl<A: 'static> Button<A> {
    pub fn new(label: &str) -> Button<A> {
        Button {
            label: Text::new(label).boxed(),
            on_click: None,
        }
    }
}

pub struct ButtonVisual<A> {
    label: Node<Box<dyn Visual<A>>>,
    on_click: Option<A>,
}

impl<A> Visual<A> for ButtonVisual<A> {
    fn paint(&mut self, painter: &mut Painter, layout: &PaintLayout) {
        painter.draw_button(
            layout.bounds,
            &ButtonState {
                disabled: false,
                clicked: false,
                hot: true,
            },
        );
        self.label.paint(painter, layout);
    }

    fn hit_test(&mut self, point: Point, layout: &PaintLayout) -> bool {
        unimplemented!()
    }

    fn event(&mut self, event: &Event, layout: &PaintLayout) -> Vec<A> {
        unimplemented!()
    }
}
