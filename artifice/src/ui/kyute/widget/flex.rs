use crate::ui::kyute::{layout::PaintLayout, layout::BoxConstraints, layout::Layout, layout::Offset, layout::Size, renderer::Painter, renderer::Renderer, visual::Node, visual::Visual, BoxedWidget, Widget, WidgetExt, Point};
use euclid::{Point2D, UnknownUnit};
use crate::ui::kyute::event::Event;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    pub fn cross_axis(self) -> Axis {
        match self {
            Axis::Horizontal => Axis::Vertical,
            Axis::Vertical => Axis::Horizontal,
        }
    }

    pub fn main_len(self, size: Size) -> f64 {
        match self {
            Axis::Vertical => size.height,
            Axis::Horizontal => size.width,
        }
    }

    pub fn cross_len(self, size: Size) -> f64 {
        match self {
            Axis::Vertical => size.width,
            Axis::Horizontal => size.height,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MainAxisAlignment {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceEvenly,
    SpaceAround,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CrossAxisAlignment {
    Baseline,
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MainAxisSize {
    Min,
    Max,
}

pub struct Flex<A> {
    axis: Axis,
    children: Vec<BoxedWidget<A>>,
}

impl<A: 'static> Flex<A> {
    pub fn new(main_axis: Axis) -> Self {
        Flex {
            axis: main_axis,
            children: Vec::new(),
        }
    }

    pub fn push(mut self, child: impl Widget<A> + 'static) -> Self {
        self.children.push(child.boxed());
        self
    }
}

impl<A: 'static> Widget<A> for Flex<A> {
    type Visual = FlexVisual<A>;

    fn layout(mut self, renderer: &Renderer, constraints: &BoxConstraints) -> Node<FlexVisual<A>> {
        let axis = self.axis;

        let child_nodes: Vec<_> = self
            .children
            .drain(..)
            .map(|c| c.layout(renderer, constraints))
            .collect();

        let max_cross_axis_len = child_nodes
            .iter()
            .map(|s| axis.cross_len(s.layout.size()))
            .fold(0.0, f64::max);

        // preferred size of this flex: max size in axis direction, max elem width in cross-axis direction
        let cross_axis_len = match self.axis {
            Axis::Vertical => constraints.constrain_width(max_cross_axis_len),
            Axis::Horizontal => constraints.constrain_height(max_cross_axis_len),
        };

        // distribute children
        let mut distributed = Vec::with_capacity(child_nodes.len());
        let mut x = 0.0;
        for mut child in child_nodes {
            let len = axis.main_len(child.layout.size);
            // offset children
            match self.axis {
                Axis::Vertical => child.layout.offset += Offset::new(0.0, x),
                Axis::Horizontal => child.layout.offset += Offset::new(x, 0.0),
            };
            distributed.push(child);
            x += len;
        }

        let size = match self.axis {
            Axis::Vertical => Size::new(cross_axis_len, constraints.max_height()),
            Axis::Horizontal => Size::new(constraints.max_width(), cross_axis_len),
        };

        let layout = Layout::new(size);
        Node::new(
            layout,
            FlexVisual {
                children: distributed,
            },
        )
    }
}

pub struct FlexVisual<A> {
    children: Vec<Node<Box<dyn Visual<A>>>>,
}

impl<A> Visual<A> for FlexVisual<A> {
    fn paint(&mut self, painter: &mut Painter, layout: &PaintLayout) {
        painter.draw_panel_background(layout.bounds);

        for child in self.children.iter_mut() {
            child.paint(painter, layout)
        }
    }

    fn hit_test(&mut self, _point: Point, _layout: &PaintLayout) -> bool {
        unimplemented!()
    }

    fn event(&mut self, _event: &Event, _layout: &PaintLayout) -> Vec<A> {
        unimplemented!()
    }
}
