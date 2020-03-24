//! `Widget` base trait and built-in widgets.
pub mod align;
pub mod baseline;
pub mod button;
pub mod dummy;
pub mod expand;
pub mod flex;
pub mod id;
pub mod map;
pub mod text;

// re-export common widgets

pub use baseline::Baseline;
pub use button::Button;
pub use dummy::DummyWidget;
pub use expand::Expand;
pub use flex::Axis;
pub use flex::Flex;
pub use map::Map;
pub use text::Text;

use crate::ui::kyute::layout::BoxConstraints;
use crate::ui::kyute::renderer::Renderer;
use crate::ui::kyute::visual::Node;
use crate::ui::kyute::visual::Visual;


/// Trait representing a widget before layout.
///
/// First, the user builds a tree of [`Widget`]s which represents the user interface. Then, the
/// widgets are laid out by calling [`Widget::layout`], which consumes the widgets and produces a tree
/// of [`Node`]s, which represent a tree of laid-out visual elements on the screen.
///
/// ## Details
///
/// The tree of [`Node`]s can be cached and reused to handle events and repaints, as long as the
/// layout does not need to changed. In contrast, the widget tree is much more short-lived, and thus
/// can easily borrow things from the application.
///
/// This is useful for widgets that create child widgets on-demand, based on layout information or
/// retained state: an example of this would be list views, which typically
/// only display a subset of the elements at a time, depending on the scroll position and the
/// available size. For lists with a lot of elements, it can be wasteful to
/// create a child widget for every element in the list up front when we know that most of them will
/// be discarded during layout. To solve this, we can pass a "widget-provider" object (typically,
/// a closure) from which the list widget could request widgets "on-demand". However, in most cases,
/// widgets are generated from application data, which means that the provider would need to borrow
/// the data to create the child widget. This is main reason behind the distinction between Nodes
/// and Widgets: if there was only one retained tree, it would borrow the application state for too
/// long, making the usage impractical.
///
/// See also [Inside Flutter - Building widgets on demand](https://flutter.dev/docs/resources/inside-flutter#building-widgets-on-demand).

pub trait Widget<A: 'static> {
    /// The type of the visual that this widget produces.
    type Visual: Visual<A> + 'static;

    /// Performs layout, consuming the widget.
    fn layout(self, renderer: &Renderer, constraints: &BoxConstraints) -> Node<Self::Visual>;
}

/// A widget wrapped in a box, that produce a visual wrapped in a box as well.
pub type BoxedWidget<A> = Box<dyn Widget<A, Visual = Box<dyn Visual<A>>>>;

/// Widget wrapper that erases the type of the visual.
struct VisualBoxWrapper<W> {
    inner: W,
}

impl<A: 'static, W: Widget<A>> Widget<A> for VisualBoxWrapper<W> {
    type Visual = Box<dyn Visual<A>>; // inferred: Box<dyn Visual<A> + 'static>

    fn layout(self, renderer: &Renderer, constraints: &BoxConstraints) -> Node<Self::Visual> {
        let node = self.inner.layout(renderer, constraints);
        Node::new(node.layout, Box::new(node.visual))
    }
}

/// Extension methods for [`Widget`].
pub trait WidgetExt<A: 'static>: Widget<A> {
    /// TODO
    fn map<B, F>(self, f: F) -> Map<A, Self, F>
    where
        Self: Sized,
        F: Fn(A) -> B,
    {
        Map::new(self, f)
    }

    /// Turns this widget into a type-erased boxed representation.
    fn boxed<'a>(self) -> Box<dyn Widget<A, Visual = Box<dyn Visual<A>>> + 'a>
    where
        Self: Sized + 'a,
    {
        Box::new(VisualBoxWrapper { inner: self })
    }
}

impl<A: 'static, W: Widget<A>> WidgetExt<A> for W {}
