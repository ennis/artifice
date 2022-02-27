use crate::{
    application::{AppCtx, ExtEvent},
    bloom::Bloom,
    cache,
    cache::Key,
    call_id::CallId,
    event::{InputState, PointerEvent, PointerEventKind},
    region::Region,
    style::VisualState,
    widget::{Align, ConstrainedBox},
    Alignment, BoxConstraints, EnvKey, Environment, Event, InternalEvent, Measurements, Offset, Point, Rect, Size,
};
use kyute_macros::composable;
use kyute_shell::{
    graal,
    graal::{ash::vk, BufferId, ImageId},
    winit::{event_loop::EventLoopWindowTarget, window::WindowId},
};
use std::{
    cell::Cell,
    fmt,
    hash::Hash,
    ops::{Deref, DerefMut, RangeBounds},
    sync::Arc,
};
use tracing::{trace, warn};

pub const SHOW_DEBUG_OVERLAY: EnvKey<bool> = EnvKey::new("kyute.show_debug_overlay");

/// Context passed to widgets during the layout pass.
///
/// See [`Widget::layout`].
pub struct LayoutCtx {
    pub scale_factor: f64,
    changed: bool,
}

impl LayoutCtx {
    pub fn new(scale_factor: f64) -> LayoutCtx {
        LayoutCtx {
            scale_factor,
            changed: false,
        }
    }

    pub fn round_to_pixel(&self, dip_length: f64) -> f64 {
        (dip_length * self.scale_factor).round()
    }
}

// TODO make things private
pub struct PaintCtx<'a> {
    pub canvas: &'a mut skia_safe::Canvas,
    pub id: Option<WidgetId>,
    pub window_bounds: Rect,
    pub focus: Option<WidgetId>,
    pub pointer_grab: Option<WidgetId>,
    pub hot: Option<WidgetId>,
    pub inputs: &'a InputState,
    pub scale_factor: f64,
    pub invalid: &'a Region,
    pub hover: bool,
    pub measurements: Measurements,
    pub active: bool,
}

impl<'a> PaintCtx<'a> {
    /// Returns the bounds of the node.
    pub fn bounds(&self) -> Rect {
        // FIXME: is the local origin always on the top-left corner?
        Rect::new(Point::origin(), self.window_bounds.size)
    }

    /// Returns the measurements computed during layout.
    pub fn measurements(&self) -> Measurements {
        self.measurements
    }

    /// Returns whether the cursor is hovering the widget.
    pub fn is_hovering(&self) -> bool {
        self.hover
    }

    /// Returns whether the widget has focus.
    pub fn has_focus(&self) -> bool {
        if let Some(id) = self.id {
            self.focus == Some(id)
        } else {
            false
        }
    }

    /// Returns whether the widget is active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Returns the current widget visual state (a bitfield summary of `is_hovering`, `has_focus`, etc.)
    pub fn visual_state(&self) -> VisualState {
        let mut state = VisualState::default();
        if self.is_hovering() {
            state |= VisualState::HOVER;
        }
        if self.has_focus() {
            state |= VisualState::FOCUS;
        }
        if self.is_active() {
            state |= VisualState::ACTIVE;
        }
        state
    }

    /*/// Returns the size of the node.
    pub fn size(&self) -> Size {
        self.window_bounds.size
    }

    pub fn is_hovering(&self) -> bool {
        self.hover
    }

    pub fn is_focused(&self) -> bool {
        self.focus == Some(self.node_id)
    }

    pub fn is_capturing_pointer(&self) -> bool {
        self.pointer_grab == Some(self.node_id)
    }*/
}

#[derive(Debug, Default)]
pub struct GpuResourceReferences {
    pub images: Vec<ImageAccess>,
    pub buffers: Vec<BufferAccess>,
}

impl GpuResourceReferences {
    pub fn new() -> GpuResourceReferences {
        GpuResourceReferences {
            images: vec![],
            buffers: vec![],
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct EventResult {
    pub handled: bool,
    pub relayout: bool,
    pub redraw: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct WindowInfo {
    pub scale_factor: f64,
}

/// Global state related to focus and pointer grab.
#[derive(Clone, Debug, Default)]
pub struct FocusState {
    pub(crate) focus: Option<WidgetId>,
    pub(crate) pointer_grab: Option<WidgetId>,
    pub(crate) hot: Option<WidgetId>,
    /// Target of popup menu events
    pub(crate) popup_target: Option<WidgetId>,
}

/*impl FocusState {
    pub fn new() -> FocusState {
        FocusState {
            focus: None,
            pointer_grab: None,
            hot: None,
            popup_target: None,
        }
    }
}*/

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum HitTestResult {
    Failed,
    Passed,
    Skipped,
}

/// Helper function to perform hit-test of a pointer event in the given bounds.
///
/// Returns:
/// - Skipped: if the hit test was skipped, because the kind of pointer event ignores hit test (e.g. pointerout)
/// - Passed:  if the pointer position fell in the given bounds
/// - Failed:  otherwise
fn hit_test_helper(
    pointer_event: &PointerEvent,
    bounds: Rect,
    id: Option<WidgetId>,
    pointer_grab: Option<WidgetId>,
) -> HitTestResult {
    if pointer_event.kind == PointerEventKind::PointerOut {
        // pointer out events are exempt from hit-test: if the pointer leaves
        // the parent widget, we also want the child elements to know that
        HitTestResult::Skipped
    } else {
        if pointer_grab.is_some() && pointer_grab == id {
            HitTestResult::Skipped
        } else {
            if bounds.contains(pointer_event.position) {
                HitTestResult::Passed
            } else {
                HitTestResult::Failed
            }
        }
    }
}

pub struct EventCtx<'a> {
    pub(crate) app_ctx: &'a mut AppCtx,
    pub(crate) event_loop: &'a EventLoopWindowTarget<ExtEvent>,
    pub(crate) parent_window: Option<&'a mut kyute_shell::window::Window>,
    pub(crate) focus_state: &'a mut FocusState,
    pub(crate) window_position: Point,
    pub(crate) scale_factor: f64,
    pub(crate) id: Option<WidgetId>,
    pub(crate) handled: bool,
    pub(crate) relayout: bool,
    pub(crate) redraw: bool,
    active: Option<bool>,
}

impl<'a> EventCtx<'a> {
    /// Creates the root `EventCtx`
    fn new(
        app_ctx: &'a mut AppCtx,
        focus_state: &'a mut FocusState,
        event_loop: &'a EventLoopWindowTarget<ExtEvent>,
        id: Option<WidgetId>,
    ) -> EventCtx<'a> {
        EventCtx {
            app_ctx,
            event_loop,
            parent_window: None,
            focus_state,
            window_position: Default::default(),
            scale_factor: 1.0,
            id,
            handled: false,
            relayout: false,
            redraw: false,
            active: None,
        }
    }

    /// Creates a new `EventCtx` to propagate events in a subwindow.
    pub(crate) fn new_subwindow<'b>(
        parent: &'b mut EventCtx,
        scale_factor: f64,
        window: &'b mut kyute_shell::window::Window,
        focus_state: &'b mut FocusState,
    ) -> EventCtx<'b>
    where
        'a: 'b,
    {
        EventCtx {
            app_ctx: parent.app_ctx,
            event_loop: parent.event_loop,
            parent_window: Some(window),
            focus_state,
            // reset window pos because we're entering a child window
            window_position: Point::origin(),
            scale_factor,
            id: parent.id,
            handled: false,
            relayout: false,
            redraw: false,
            active: None,
        }
    }

    /// Performs hit-testing of the specified event in the given sub-bounds.
    ///
    /// The behavior of hit-testing is as follows:
    /// - if the event is not a pointer event, the hit-test passes automatically
    /// - otherwise, do the pointer event hit-test
    ///     - TODO details
    ///
    /// The function returns whether the hit-test passed, and, if it was successful and the event
    /// was a pointer event, the pointer event with coordinates relative to the given sub-bounds.
    pub fn hit_test(&mut self, pointer_event: &PointerEvent, bounds: Rect) -> HitTestResult {
        hit_test_helper(pointer_event, bounds, self.id, self.focus_state.pointer_grab)
    }

    /// Returns the parent widget ID.
    pub fn widget_id(&self) -> Option<WidgetId> {
        self.id
    }

    pub fn set_state<T: 'static>(&mut self, key: Key<T>, value: T) {
        self.app_ctx.cache.set_state(key, value)
    }

    pub fn register_window(&mut self, window_id: WindowId) {
        if let Some(id) = self.id {
            self.app_ctx.register_window_widget(window_id, id);
        } else {
            warn!("register_window: the widget registering the window must have an ID")
        }
    }

    /// Returns the bounds of the current widget.
    // TODO in what space?
    pub fn bounds(&self) -> Rect {
        todo!()
    }

    /// Requests a redraw of the current node and its children.
    pub fn request_redraw(&mut self) {
        self.redraw = true;
    }

    pub fn request_recomposition(&mut self) {
        todo!()
    }

    /// Requests a relayout of the current widget.
    pub fn request_relayout(&mut self) {
        self.relayout = true;
    }

    /// Requests that the current node grabs all pointer events in the parent window.
    pub fn capture_pointer(&mut self) {
        if let Some(id) = self.id {
            self.focus_state.pointer_grab = Some(id);
        } else {
            warn!("capture_pointer: the widget capturing the pointer must have an ID")
        }
    }

    /// Returns whether the current node is capturing the pointer.
    #[must_use]
    pub fn is_capturing_pointer(&self) -> bool {
        if let Some(id) = self.id {
            self.focus_state.pointer_grab == Some(id)
        } else {
            false
        }
    }

    /// Releases the pointer grab, if the current node is holding it.
    pub fn release_pointer(&mut self) {
        if let Some(id) = self.id {
            if self.focus_state.pointer_grab == Some(id) {
                trace!("releasing pointer grab");
            } else {
                warn!("pointer capture release requested but the current widget isn't capturing the pointer");
            }
        } else {
            warn!("capture_pointer: the calling widget must have an ID")
        }
    }

    /// Acquires the focus.
    pub fn request_focus(&mut self) {
        if let Some(id) = self.id {
            self.focus_state.focus = Some(id);
        } else {
            warn!("request_focus: the calling widget must have an ID")
        }
    }

    /// Returns whether the current node has the focus.
    #[must_use]
    pub fn has_focus(&self) -> bool {
        if let Some(id) = self.id {
            self.focus_state.focus == Some(id)
        } else {
            false
        }
    }

    pub fn track_popup_menu(&mut self, menu: kyute_shell::Menu, at: Point) {
        if let Some(id) = self.id {
            self.focus_state.popup_target = Some(id);
            let at = ((at.x * self.scale_factor) as i32, (at.y * self.scale_factor) as i32);
            self.parent_window
                .as_mut()
                .expect("EventCtx::track_popup_menu called without a parent window")
                .show_context_menu(menu, at);
        } else {
            warn!("track_popup_menu: the calling widget must have an ID")
        }
    }

    /// Signals that the passed event was handled and should not bubble up further.
    pub fn set_handled(&mut self) {
        self.handled = true;
    }

    /// Signals that the widget became active or inactive.
    pub fn set_active(&mut self, active: bool) {
        self.active = Some(active);
    }

    #[must_use]
    pub fn handled(&self) -> bool {
        self.handled
    }
}

pub struct WindowPaintCtx {}

#[derive(Debug)]
pub struct ImageAccess {
    pub id: ImageId,
    pub initial_layout: vk::ImageLayout,
    pub final_layout: vk::ImageLayout,
    pub access_mask: vk::AccessFlags,
    pub stage_mask: vk::PipelineStageFlags,
}

#[derive(Debug)]
pub struct BufferAccess {
    pub id: BufferId,
    pub access_mask: vk::AccessFlags,
    pub stage_mask: vk::PipelineStageFlags,
}

pub struct GpuFrameCtx<'a, 'b> {
    /// graal context in frame recording state.
    pub(crate) frame: &'b mut graal::Frame<'a, ()>,
    pub(crate) resource_references: GpuResourceReferences,
    pub(crate) measurements: Measurements,
    pub(crate) scale_factor: f64,
}

impl<'a, 'b> GpuFrameCtx<'a, 'b> {
    /// Returns a ref to the frame.
    pub fn frame(&mut self) -> &mut graal::Frame<'a, ()> {
        self.frame
    }

    #[must_use]
    pub fn measurements(&self) -> Measurements {
        self.measurements
    }

    /// Registers an image that will be accessed during paint.
    pub fn reference_paint_image(
        &mut self,
        id: ImageId,
        access_mask: vk::AccessFlags,
        stage_mask: vk::PipelineStageFlags,
        initial_layout: vk::ImageLayout,
        final_layout: vk::ImageLayout,
    ) {
        self.resource_references.images.push(ImageAccess {
            id,
            initial_layout,
            final_layout,
            access_mask,
            stage_mask,
        })
    }

    /// Registers a buffer that will be accessed during paint.
    pub fn reference_paint_buffer(
        &mut self,
        id: BufferId,
        access_mask: vk::AccessFlags,
        stage_mask: vk::PipelineStageFlags,
    ) {
        self.resource_references.buffers.push(BufferAccess {
            id,
            access_mask,
            stage_mask,
        })
    }
}

/// Trait that defines the behavior of a widget.
pub trait Widget {
    /// Returns the widget identity.
    fn widget_id(&self) -> Option<WidgetId>;

    /// Implement to give a debug name to your widget. Used only for debugging.
    fn debug_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    /// Propagates an event through the widget hierarchy.
    fn event(&self, ctx: &mut EventCtx, event: &mut Event, env: &Environment);

    /// Measures this widget and layouts the children of this widget.
    fn layout(&self, ctx: &mut LayoutCtx, constraints: BoxConstraints, env: &Environment) -> Measurements;

    /// Paints the widget in the given context.
    fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, env: &Environment);

    /// Called only for native window widgets.
    fn window_paint(&self, _ctx: &mut WindowPaintCtx) {}

    /// Called for custom GPU operations
    fn gpu_frame<'a, 'b>(&'a self, _ctx: &mut GpuFrameCtx<'a, 'b>) {}
}

/// Arc'd widgets.
impl<T: Widget + ?Sized> Widget for Arc<T> {
    fn widget_id(&self) -> Option<WidgetId> {
        Widget::widget_id(&**self)
    }

    fn debug_name(&self) -> &str {
        Widget::debug_name(&**self)
    }

    fn event(&self, ctx: &mut EventCtx, event: &mut Event, env: &Environment) {
        Widget::event(&**self, ctx, event, env)
    }

    fn layout(&self, ctx: &mut LayoutCtx, constraints: BoxConstraints, env: &Environment) -> Measurements {
        Widget::layout(&**self, ctx, constraints, env)
    }

    fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, env: &Environment) {
        Widget::paint(&**self, ctx, bounds, env)
    }

    fn window_paint(&self, ctx: &mut WindowPaintCtx) {
        Widget::window_paint(&**self, ctx)
    }

    fn gpu_frame<'a, 'b>(&'a self, ctx: &mut GpuFrameCtx<'a, 'b>) {
        Widget::gpu_frame(&**self, ctx)
    }
}

/// Extension methods on widgets.
pub trait WidgetExt: Widget + Sized + 'static {
    /// Wraps the widget in a `ConstrainedBox` that constrains the width of the widget.
    #[composable]
    fn constrain_width(self, width: impl RangeBounds<f64>) -> ConstrainedBox<Self> {
        ConstrainedBox::new(BoxConstraints::new(width, ..), self)
    }

    /// Wraps the widget in a `ConstrainedBox` that constrains the height of the widget.
    #[composable]
    fn constrain_height(self, height: impl RangeBounds<f64>) -> ConstrainedBox<Self> {
        ConstrainedBox::new(BoxConstraints::new(.., height), self)
    }

    /// Wraps the widget in a `ConstrainedBox` that constrains the width of the widget.
    #[composable]
    fn fix_width(self, width: f64) -> ConstrainedBox<Self> {
        ConstrainedBox::new(BoxConstraints::new(width..width, ..), self)
    }

    /// Wraps the widget in a `ConstrainedBox` that constrains the height of the widget.
    #[composable]
    fn fix_height(self, height: f64) -> ConstrainedBox<Self> {
        ConstrainedBox::new(BoxConstraints::new(.., height..height), self)
    }
    /// Wraps the widget in a `ConstrainedBox` that constrains the size of the widget.
    #[composable]
    fn fix_size(self, size: Size) -> ConstrainedBox<Self> {
        ConstrainedBox::new(BoxConstraints::tight(size), self)
    }

    /// Centers the widget in the available space.
    #[composable]
    fn centered(self) -> Align<Self> {
        Align::new(Alignment::CENTER, self)
    }

    /// Aligns the widget in the available space.
    #[composable]
    fn aligned(self, alignment: Alignment) -> Align<Self> {
        Align::new(alignment, self)
    }
}

impl<W: Widget + 'static> WidgetExt for W {}

/// ID of a node in the tree.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
#[repr(transparent)]
pub struct WidgetId(CallId);

impl WidgetId {
    pub(crate) fn from_call_id(call_id: CallId) -> WidgetId {
        WidgetId(call_id)
    }

    #[composable]
    pub fn here() -> WidgetId {
        WidgetId(cache::current_call_id())
    }
}

impl fmt::Debug for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:04X}", self.0.to_u64())
    }
}

pub type WidgetFilter = Bloom<WidgetId>;

/*// FIXME: replace with just a WidgetId
#[derive(Clone, Debug)]
pub struct WidgetIdentity {
    /// Unique ID of the widget.
    id: WidgetId,

    /// The revision in which the WidgetPod was created.
    // TODO remove this? move to widgetpod?
    created: usize,

    /// Debugging: flag indicating whether this Widget was recreated since the last
    /// debug paint.
    // TODO remove?
    created_since_debug_paint: Cell<bool>,
}

impl WidgetIdentity {
    /// Creates a new `WidgetState` representing the state of a widget anchored at the call site.
    /// TODO fix the doc
    #[composable]
    pub fn new() -> WidgetIdentity {
        // derive the widget identity (it's ID) from the current call ID (i.e. the call stack).
        let id = WidgetId::from_call_id(cache::current_call_id());

        let created = cache::revision();
        WidgetIdentity {
            id,
            created,
            created_since_debug_paint: Cell::new(true),
            //child_filter: Cell::new(None),
        }
    }
}*/

#[derive(Copy, Clone, Debug, Hash)]
struct LayoutResult {
    constraints: BoxConstraints,
    measurements: Measurements,
}

struct WidgetPodState {
    /// Unique ID of the widget, if it has one.
    id: Option<WidgetId>,

    /// Position of this widget relative to its parent. Set by `WidgetPod::set_child_offset`.
    offset: Cell<Offset>,

    /// Indicates that this widget should be repainted.
    /// Set by `layout` if the layout has changed somehow, after event handling if `EventCtx::request_redraw` was called,
    /// and by `set_child_offset`.
    paint_invalid: Cell<bool>,

    /// Layout result.
    layout_result: Cell<Option<LayoutResult>>,

    /// Any pointer hovering this widget
    // FIXME: handle multiple pointers?
    // FIXME: this is destroyed on recomp, probably not what we want
    // FIXME: never mind that, this is invalidated on **relayouts**, this is totally broken
    pointer_over: Cell<bool>,

    /// Whether the widget is active.
    // FIXME: don't reset on recomp
    active: Cell<bool>,

    /// Indicates that the children of this widget have been initialized.
    ///
    /// Reset on recomp, by design: there may be new children.
    children_initialized: Cell<bool>,

    /// Bloom filter to filter child widgets.
    child_filter: Cell<Option<WidgetFilter>>,
}

impl WidgetPodState {
    /// Sets the offset of this widget relative to its parent.
    pub fn set_child_offset(&self, offset: Offset) {
        self.offset.set(offset);
        self.paint_invalid.set(true);
    }
}

/// A container for a widget that gives it an identity (a `WidgetId`), derived from its position in
/// the call tree.
/// TODO fix the docs.
pub struct WidgetPod<T: ?Sized = dyn Widget> {
    state: WidgetPodState,
    widget: T,
}

impl<T: Widget + ?Sized> WidgetPod<T> {
    fn compute_child_filter(&self, parent_ctx: &mut EventCtx, env: &Environment) -> Bloom<WidgetId> {
        if let Some(filter) = self.state.child_filter.get() {
            // already computed
            filter
        } else {
            //tracing::trace!("computing child filter");
            let mut filter = Default::default();
            self.do_event(
                parent_ctx,
                &mut Event::Internal(InternalEvent::UpdateChildFilter { filter: &mut filter }),
                env,
            );
            self.state.child_filter.set(Some(filter));
            filter
        }
    }

    /// Returns whether this widget may contain the specified widget as a child (direct or not).
    fn may_contain(&self, widget: WidgetId) -> bool {
        if let Some(filter) = self.state.child_filter.get() {
            filter.may_contain(&widget)
        } else {
            warn!("`may_contain` called but child filter not initialized");
            true
        }
    }

    /// Used internally by `event`. In charge of calling the `event` method on the widget with
    /// the child `EventCtx`, and handling its result.
    fn do_event(&self, parent_ctx: &mut EventCtx, event: &mut Event, env: &Environment) {
        let offset = self.state.offset.get();
        let window_position = parent_ctx.window_position + offset;
        let mut ctx = EventCtx {
            app_ctx: parent_ctx.app_ctx,
            event_loop: parent_ctx.event_loop,
            parent_window: parent_ctx.parent_window.as_deref_mut(),
            focus_state: parent_ctx.focus_state,
            window_position,
            scale_factor: parent_ctx.scale_factor,
            id: self.id(),
            handled: false,
            relayout: false,
            redraw: false,
            active: None,
        };
        self.widget.event(&mut ctx, event, env);

        // -- update widget state from ctx

        // relayout and redraws
        if ctx.relayout {
            //tracing::trace!(widget_id = ?self.state.id, "requested relayout");
            // relayout requested by the widget: invalidate cached measurements and offset
            self.state.offset.set(Offset::zero());
            self.state.layout_result.set(None);
            self.state.paint_invalid.set(true);
        } else if ctx.redraw {
            //tracing::trace!(widget_id = ?self.state.id, "requested redraw");
            self.state.paint_invalid.set(true);
        }

        // active flag
        if let Some(active) = ctx.active {
            self.state.active.set(active);
        }

        // -- propagate results to parent
        parent_ctx.relayout |= ctx.relayout;
        parent_ctx.redraw |= ctx.redraw;
        parent_ctx.handled = ctx.handled;
    }

    /// Called to measure this widget and layout the children of this widget.
    pub fn layout(&self, ctx: &mut LayoutCtx, constraints: BoxConstraints, env: &Environment) -> Measurements {
        // FIXME also check the environment when checking the validity of a cached layout.
        // if the layout that we calculated is valid, return it
        // FIXME grids call layout twice, which makes this kind of caching useless
        if let Some(layout) = self.state.layout_result.get() {
            if layout.constraints == constraints {
                //trace!("using cached layout");
                return layout.measurements;
            } /*else {
                  trace!("constraints mismatch {:?} {:?}", layout.constraints, constraints);
              }*/
        }

        //trace!("recalculating layout");
        let measurements = self.widget.layout(ctx, constraints, env);
        /*tracing::trace!(
            "layout[{}-{:?}]: {:?}",
            self.widget.debug_name(),
            self.state.id,
            measurements
        );*/
        let result = LayoutResult {
            constraints,
            measurements,
        };

        self.state.layout_result.set(Some(result));
        self.state.paint_invalid.set(true);
        ctx.changed = true;
        measurements
    }

    pub fn paint(&self, parent_ctx: &mut PaintCtx, _bounds: Rect, env: &Environment) {
        /*if !self.0.paint_invalid.get() {
            // no need to repaint
            return;
        }*/

        let offset = self.state.offset.get();
        let measurements = if let Some(layout_result) = self.state.layout_result.get() {
            layout_result.measurements
        } else {
            tracing::warn!(id=?self.id(), "`paint` called with invalid layout");
            return;
        };
        let size = measurements.size();
        // bounds of this widget in window space
        let window_bounds = Rect::new(parent_ctx.window_bounds.origin + offset, size);
        if !parent_ctx.invalid.intersects(window_bounds) {
            //tracing::trace!("not repainting valid region");
            // not invalidated, no need to redraw
            return;
        }

        /*let _span = trace_span!(
            "paint",
            ?self.id,
            ?offset,
            ?measurements,
        ).entered();*/
        // trace!(?ctx.scale_factor, ?ctx.inputs.pointers, ?window_bounds, "paint");

        let hover = parent_ctx
            .inputs
            .pointers
            .iter()
            .any(|(_, state)| window_bounds.contains(state.position));

        parent_ctx.canvas.save();
        parent_ctx
            .canvas
            .translate(skia_safe::Vector::new(offset.x as f32, offset.y as f32));

        {
            let mut child_ctx = PaintCtx {
                canvas: parent_ctx.canvas,
                window_bounds,
                focus: parent_ctx.focus,
                pointer_grab: parent_ctx.pointer_grab,
                hot: parent_ctx.hot,
                inputs: parent_ctx.inputs,
                scale_factor: parent_ctx.scale_factor,
                id: self.id(),
                hover,
                invalid: &parent_ctx.invalid,
                measurements,
                active: self.state.active.get(),
            };
            self.widget.paint(&mut child_ctx, Rect::new(Point::origin(), size), env);
        }

        /*if !env.get(SHOW_DEBUG_OVERLAY).unwrap_or_default() {
            use crate::styling::*;
            use kyute_shell::{drawing::ToSkia, skia as sk};

            if self.state.created_since_debug_paint.take() {
                ctx.draw_styled_box(
                    measurements.bounds,
                    rectangle().with(
                        border(1.0)
                            .inside(0.0)
                            .brush(Color::new(0.9, 0.8, 0.0, 1.0)),
                    ),
                    env,
                );
                ctx.canvas.draw_line(
                    Point::new(0.5, 0.5).to_skia(),
                    Point::new(6.5, 0.5).to_skia(),
                    &sk::Paint::new(Color::new(1.0, 0.0, 0.0, 1.0).to_skia(), None),
                );
                ctx.canvas.draw_line(
                    Point::new(0.5, 0.5).to_skia(),
                    Point::new(0.5, 6.5).to_skia(),
                    &sk::Paint::new(Color::new(0.0, 1.0, 0.0, 1.0).to_skia(), None),
                );

                {
                    let w = measurements.bounds.width() as sk::scalar;
                    let mut font: sk::Font = sk::Font::new(sk::Typeface::default(), Some(10.0));
                    font.set_edging(sk::font::Edging::Alias);
                    let text = format!("{}", self.state.created);
                    let text_blob =
                        sk::TextBlob::from_str(&text, &font).unwrap();
                    let text_paint: sk::Paint =
                        sk::Paint::new(sk::Color4f::new(0.0, 0.0, 0.0, 1.0), None);
                    let bg_paint: sk::Paint =
                        sk::Paint::new(sk::Color4f::new(0.9, 0.8, 0.0, 1.0), None);
                    let (_, bounds) = font.measure_str(&text, Some(&text_paint));
                    ctx.canvas.draw_rect(
                        sk::Rect::new(w - bounds.width(), 0.0, w, bounds.height()),
                        &bg_paint,
                    );
                    ctx.canvas
                        .draw_text_blob(text_blob, (w - bounds.width(), -bounds.y()), &text_paint);
                    //let bounds = Rect::from_skia(bounds);
                }
            }
        }*/

        parent_ctx.canvas.restore();
        self.state.paint_invalid.set(false);
    }

    /// Propagates an event to the wrapped widget.
    pub fn event(&self, parent_ctx: &mut EventCtx, event: &mut Event, env: &Environment) {
        if parent_ctx.handled {
            tracing::warn!("event already handled");
            return;
        }

        // first, ensure that the child filter has been computed and the child widgets are initialized
        self.compute_child_filter(parent_ctx, env);

        // ---- Handle internal events (routing mostly) ----
        match *event {
            Event::Internal(InternalEvent::RouteWindowEvent {
                target,
                event: ref mut window_event,
            }) => {
                // routing of `winit::WindowEvent`s to the corresponding window widget.
                if Some(target) == self.state.id {
                    self.do_event(parent_ctx, &mut Event::WindowEvent(window_event.clone()), env);
                } else if self.may_contain(target) {
                    self.do_event(parent_ctx, event, env);
                }
                return;
            }
            Event::Internal(InternalEvent::RouteEvent { target, ref mut event }) => {
                if Some(target) == self.state.id {
                    // we reached the target, unwrap the inner event and restart
                    self.event(parent_ctx, event, env);
                } else if self.may_contain(target) {
                    self.do_event(parent_ctx, event, env);
                }
                return;
            }
            Event::Internal(InternalEvent::RoutePointerEvent {
                target,
                event: ref mut pointer_event,
            }) => {
                // routed pointer events follow the same logic as routed events (the only difference is the behavior of hit-test)
                if Some(target) == self.state.id {
                    //trace!("pointer event reached {:?}", target);
                    self.event(parent_ctx, &mut Event::Pointer(*pointer_event), env);
                } else if self.may_contain(target) {
                    event.with_local_coordinates(self.state.offset.get(), |event| {
                        self.do_event(parent_ctx, event, env);
                    });
                }
                return;
            }
            Event::Internal(InternalEvent::Traverse { ref mut widgets }) => {
                // T: ?Sized
                // This is problematic: it must clone self, and thus we must either have T == dyn Widget or T:Sized
                //widgets.push(WidgetPod(self.state.this.upgrade().unwrap()));
            }
            Event::Internal(InternalEvent::RouteRedrawRequest(target)) => {
                if Some(target) == self.state.id {
                    self.do_event(parent_ctx, &mut Event::WindowRedrawRequest, env);
                } else if self.may_contain(target) {
                    self.do_event(parent_ctx, event, env);
                }
                return;
            }
            Event::Internal(InternalEvent::UpdateChildFilter { ref mut filter }) => {
                if let Some(id) = self.state.id {
                    filter.add(&id);
                }
                let child_filter = self.compute_child_filter(parent_ctx, env);
                filter.extend(&child_filter);
                return;
            }
            Event::Initialize => {
                // directly pass to widget (bypass hit test)
                self.do_event(parent_ctx, event, env);
                return;
            }
            /*Event::Initialize => {
                // Widgets without identity do not receive targeted events, so assume that init=true for those.
                let init = self
                    .widget
                    .widget_identity()
                    .map(|w| w.initialized.get())
                    .unwrap_or(true);
                let child_init = self.state.children_initialized.get();

                trace!(
                    "{} {:?}  -->  {:?} init={}, child_init={}",
                    self.widget.debug_name(),
                    self.id(),
                    event,
                    init,
                    child_init
                );

                // Widget receiving an `Initialize` or `RouteInitialize` event.
                // If the widget is not yet initialized (init=false), then initialize it (by sending
                // an `Event::Initialize` to ourselves). This will also initialize the children because it is the
                // responsibility of the widget to propagate the event to its children.
                //
                // Otherwise:
                //  - if the widget is initialized, but not its children (child_init): propagate the RouteInitialize
                //    event so that the uninitialized children have a chance to initialize themselves.
                //  - if both are initialized, do nothing
                match (init, child_init) {
                    (false, _) => self.do_event(parent_ctx, &mut Event::Initialize, env),
                    (true, false) => {
                        self.do_event(parent_ctx, &mut Event::Internal(InternalEvent::RouteInitialize), env)
                    }
                    _ => {}
                }

                if let Some(identity) = self.widget.widget_identity() {
                    identity.initialized.set(true);
                }
                self.state.children_initialized.set(true);
                return;
            }*/
            _ => {}
        }

        // ---- hit-test pointer events
        let measurements = if let Some(layout_result) = self.state.layout_result.get() {
            layout_result.measurements
        } else {
            tracing::warn!("`event` called before layout ({:?})", event);
            return;
        };

        event.with_local_coordinates(self.state.offset.get(), |event| match event {
            Event::Pointer(p) => {
                match hit_test_helper(p, measurements.bounds, self.id(), parent_ctx.focus_state.pointer_grab) {
                    HitTestResult::Passed => {
                        if !self.state.pointer_over.get() {
                            self.state.pointer_over.set(true);
                            self.do_event(
                                parent_ctx,
                                &mut Event::Pointer(PointerEvent {
                                    kind: PointerEventKind::PointerOver,
                                    ..*p
                                }),
                                env,
                            );
                        }
                        self.do_event(parent_ctx, event, env);
                    }
                    HitTestResult::Failed => {
                        if self.state.pointer_over.get() {
                            self.state.pointer_over.set(false);
                            self.do_event(
                                parent_ctx,
                                &mut Event::Pointer(PointerEvent {
                                    kind: PointerEventKind::PointerOut,
                                    ..*p
                                }),
                                env,
                            );
                        }
                    }
                    HitTestResult::Skipped => {
                        self.do_event(parent_ctx, event, env);
                    }
                }
            }
            _ => {
                self.do_event(parent_ctx, event, env);
            }
        });
    }
}

/*// Unsized coercions
impl<T, U> CoerceUnsized<WidgetPod<U>> for WidgetPod<T>
where
    T: Unsize<U> + ?Sized,
    U: ?Sized,
{
}*/

impl fmt::Debug for WidgetPod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO
        f.debug_tuple("WidgetPod").finish()
    }
}

impl<T: Widget + 'static> WidgetPod<T> {
    /// Creates a new `WidgetPod` wrapping the specified widget.
    #[composable]
    pub fn new(widget: T) -> WidgetPod<T> {
        let id = widget.widget_id();

        WidgetPod {
            state: WidgetPodState {
                id,
                offset: Cell::new(Offset::zero()),
                paint_invalid: Cell::new(true),
                // we don't know if all children have been initialized
                children_initialized: Cell::new(false),
                pointer_over: Cell::new(false),
                active: Cell::new(false),
                layout_result: Cell::new(None),
                child_filter: Cell::new(None),
            },
            widget,
        }
    }
}

/*impl<T: Widget + ?Sized> Deref for WidgetPod<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T: Widget + ?Sized> DerefMut for WidgetPod<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}*/

/*// TODO remove this once we have unsized coercions
impl<T: Widget + 'static> From<WidgetPod<T>> for WidgetPod {
    fn from(other: WidgetPod<T>) -> Self {
        WidgetPod(other.0)
    }
}*/

impl<T: ?Sized + Widget> WidgetPod<T> {
    /// Returns a reference to the wrapped widget.
    pub fn widget(&self) -> &T {
        &self.widget
    }

    /// Returns the widget id.
    pub fn id(&self) -> Option<WidgetId> {
        self.state.id
    }

    /// Returns previously set child offset. See `set_child_offset`.
    pub fn child_offset(&self) -> Offset {
        self.state.offset.get()
    }

    /// TODO documentation
    /// Sets the offset of this widget relative to its parent. Should be called during widget layout.
    pub fn set_child_offset(&self, offset: Offset) {
        self.state.set_child_offset(offset);
    }

    /// Returns whether the widget should be repainted.
    pub fn invalidated(&self) -> bool {
        self.state.paint_invalid.get()
    }

    /// Computes the layout of this widget and its children. Returns the measurements, and whether
    /// the measurements have changed since last layout.
    pub fn relayout(&self, constraints: BoxConstraints, scale_factor: f64, env: &Environment) -> (Measurements, bool) {
        let mut ctx = LayoutCtx {
            scale_factor,
            changed: false,
        };
        let measurements = self.layout(&mut ctx, constraints, env);
        (measurements, ctx.changed)
    }

    /// Prepares the root `EventCtx` and calls `self.event()`.
    pub(crate) fn send_root_event(
        &self,
        app_ctx: &mut AppCtx,
        event_loop: &EventLoopWindowTarget<ExtEvent>,
        event: &mut Event,
        env: &Environment,
    ) {
        // FIXME callId?
        // The dummy `FocusState` for the root `EventCtx`. It is eventually replaced with the `FocusState`
        // managed by `Window` widgets.
        let mut dummy_focus_state = FocusState::default();
        let mut event_ctx = EventCtx::new(
            app_ctx,
            &mut dummy_focus_state,
            event_loop,
            Some(WidgetId::from_call_id(CallId(0))),
        );
        //tracing::trace!("event={:?}", event);
        self.event(&mut event_ctx, event, env);
    }

    /// Initializes and layouts the widget if necessary (propagates the `Initialize` event and
    /// calls `root_layout`.
    pub(crate) fn initialize(
        &self,
        app_ctx: &mut AppCtx,
        event_loop: &EventLoopWindowTarget<ExtEvent>,
        env: &Environment,
    ) {
        self.send_root_event(app_ctx, event_loop, &mut Event::Initialize, env);
    }

    /*pub(crate) fn root_layout(&self, app_ctx: &mut AppCtx, env: &Environment) -> bool {
        let mut ctx = LayoutCtx { changed: false };
        self.layout(
            &mut ctx,
            BoxConstraints {
                min: Size::new(0.0, 0.0),
                max: Size::new(f64::INFINITY, f64::INFINITY),
            },
            env,
        );
        ctx.changed
    }*/
}