use crate::util::model::Data;
use crate::util::model::Revision;

use crate::ui::common::BoxConstraints;
use crate::ui::common::Size;
use crate::ui::common::widgets::Map;
use crate::ui::common::platform;
use euclid::default::{Rect, Transform2D};

pub type ViewEvent<'a> = winit::event::WindowEvent<'a>;

pub trait ActionSink<A> {
    fn emit(&mut self, a: A);
}

struct ActionCollector<A> {
    actions: Vec<A>,
}

impl<A> ActionSink<A> for ActionCollector<A> {
    fn emit(&mut self, a: A) {
        self.actions.push(a);
    }
}

/// Context passed to [`View::event`].
///
/// It receives _actions_ emitted by Views.
pub struct EventCtx<'a, A> {
    actions: &'a mut dyn ActionSink<A>,
}

impl<'a, A> EventCtx<'a, A> {
    pub fn new(actions: &'a mut dyn ActionSink<A>) -> EventCtx<'a, A> {
        EventCtx { actions }
    }

    pub fn emit(&mut self, a: A) {
        self.actions.emit(a);
    }

    pub fn action_sink(&mut self) -> &mut dyn ActionSink<A> {
        self.actions
    }
}

/// Context passed to [`View::layout`].
pub struct LayoutCtx {}

/// Context passed to [`View::paint`].
pub struct PaintCtx<'a>(platform::PaintCtx<'a>);

impl<'a> PaintCtx<'a> {
    /// Runs the provided closure with a new PaintCtx which has the specified transformation
    /// applied.
    pub fn with_transform<R>(&mut self, transform: Transform2D<f64>, f: impl FnOnce(&mut PaintCtx) -> R) -> R {
        unimplemented!()
    }
}

pub trait View<S: Data> {
    type Action;

    /// Called on event.
    fn event(&mut self, e: &ViewEvent, a: &mut EventCtx<Self::Action>);

    /// Called when the ambient state has changed.
    fn update(&mut self, s: &Revision<S>);

    /// Called when it's time to paint the view.
    ///
    /// Should return true if the view is requesting another animation frame just after.
    fn paint(&mut self, state: &S, ctx: &mut PaintCtx) -> bool;

    /// Layouts the view: returns the desired size of the view given parent constraints.
    fn layout(&mut self, state: &S, ctx: &mut LayoutCtx, constraints: &BoxConstraints) -> Size;
}


pub struct CachedLayout<V> {
    /// Layout rect (relative to parent).
    layout_rect: Option<Rect<f64>>,
    /// Inner view.
    view: V,
}

impl<S,V> CachedLayout<V> where V: View<S> {
    /// Wraps the view.
    pub fn new(view: V) -> CachedLayout<V> {
        CachedLayout {
            layout_rect: None,
            view
        }
    }

    pub fn layout(&mut self, state: &S, ctx: &mut LayoutCtx, constraints: &BoxConstraints) -> Size {
        self.view.layout(state, ctx, constraints)
    }

    pub fn set_layout_rect(&mut self, rect: Rect<f64>) {
        self.layout_rect = Some(rect)
    }

    pub fn layout_rect(&self) -> Option<Rect<f64>> {
        self.layout_rect
    }

    pub fn paint(&mut self, state: &S, ctx: &mut PaintCtx) -> bool {

    }
}


pub trait ViewExt<S: Data>: View<S> {
    fn map<A, F>(self, closure: F) -> Map<Self, F>
    where
        Self: Sized,
        F: Fn(Self::Action) -> A,
    {
        Map::new(self, closure)
    }
}

impl<S: Data, V: View<S>> ViewExt<S> for V {}