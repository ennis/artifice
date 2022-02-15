//! winit-based application wrapper.
//!
//! Provides the `run_application` function that opens the main window and translates the incoming
//! events from winit into the events expected by kyute.
use crate::{core2::WidgetId, Cache, Environment, Event, InternalEvent, Widget, WidgetPod};
use kyute_shell::{
    winit,
    winit::{
        event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
        window::WindowId,
    },
};
use std::{
    collections::{hash_map::Entry, HashMap},
    mem,
    sync::Arc,
};
use tracing::warn;

/// Global application context. Contains stuff passed to all widget contexts (Event,Layout,Paint...)
pub struct AppCtx {
    /// Open windows, mapped to their corresponding widget.
    pub(crate) windows: HashMap<WindowId, WidgetId>,
    /// Main UI cache.
    ///
    /// Stores cached copies of widgets and state variables.
    pub(crate) cache: Cache,
    pub(crate) pending_events: Vec<Event<'static>>,
}

impl AppCtx {
    /// Creates a new AppCtx.
    fn new() -> AppCtx {
        AppCtx {
            windows: HashMap::new(),
            cache: Cache::new(),
            pending_events: vec![],
        }
    }

    /// Registers a widget as a native window widget.
    ///
    /// The event loop will call `window_event` whenever an event targeting the window is received.
    pub(crate) fn register_window_widget(&mut self, window_id: WindowId, widget_id: WidgetId) {
        match self.windows.entry(window_id) {
            Entry::Occupied(_) => {
                warn!("window id {:?} already registered", window_id);
            }
            Entry::Vacant(entry) => {
                entry.insert(widget_id);
            }
        }
    }

    pub fn post_event(&mut self, event: Event<'static>) {
        //tracing::trace!("post_event {:?}", &event);
        self.pending_events.push(event);
    }

    fn send_event(
        &mut self,
        root_widget: &WidgetPod,
        event_loop: &EventLoopWindowTarget<()>,
        event: Event<'static>,
        root_env: &Environment,
    ) {
        self.post_event(event);
        self.flush_pending_events(root_widget, event_loop, root_env);
    }

    fn flush_pending_events(
        &mut self,
        root_widget: &WidgetPod,
        event_loop: &EventLoopWindowTarget<()>,
        root_env: &Environment,
    ) {
        while !self.pending_events.is_empty() {
            let events = mem::take(&mut self.pending_events);
            for mut event in events {
                root_widget.send_root_event(self, event_loop, &mut event, root_env)
            }
        }
    }
}

fn eval_root_widget(
    app_ctx: &mut AppCtx,
    event_loop: &EventLoopWindowTarget<()>,
    root_env: &Environment,
    f: fn() -> Arc<WidgetPod>,
) -> Arc<WidgetPod> {
    let root_widget = app_ctx.cache.run(f);
    // ensures that all widgets have received the `Initialize` event.
    root_widget.initialize(app_ctx, event_loop, root_env);
    root_widget
}

pub fn run(ui: fn() -> Arc<WidgetPod>, env: Environment) {
    let event_loop = EventLoop::new();
    let mut app_ctx = AppCtx::new();

    // initial evaluation of the root widget in the main UI cache.
    let mut root_widget = eval_root_widget(&mut app_ctx, &event_loop, &env, ui);

    // run event loop
    event_loop.run(move |event, elwt, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            // --- WINDOW EVENT PROCESSING ---------------------------------------------------------
            winit::event::Event::WindowEvent {
                window_id,
                event: winit_event,
            } => {
                if let Some(&target) = app_ctx.windows.get(&window_id) {
                    if let Some(event) = winit_event.to_static() {
                        app_ctx.send_event(
                            &root_widget,
                            elwt,
                            Event::Internal(InternalEvent::RouteWindowEvent { target, event }),
                            &env,
                        );
                    }
                } else {
                    tracing::warn!("unregistered window id: {:?}", window_id);
                }
            }
            // --- RECOMPOSITION -------------------------------------------------------------------
            // happens after window event processing
            winit::event::Event::MainEventsCleared => {
                // Re-evaluate the root widget.
                // If no state variable in the cache has changed (because of an event), then it will simply
                // return the same root widget.

                // TODO: this is broken: recomp should run in a loop until stabilized (root cache entry not dirty)

                // 1st eval: run event handlers
                // 2nd eval: reflect new state of UI
                //tracing::trace!("1st recomp");
                eval_root_widget(&mut app_ctx, elwt, &env, ui);
                //tracing::trace!("2nd recomp");
                root_widget = eval_root_widget(&mut app_ctx, elwt, &env, ui);
            }
            // --- REPAINT -------------------------------------------------------------------------
            // happens after recomposition
            winit::event::Event::RedrawRequested(window_id) => {
                if let Some(&target) = app_ctx.windows.get(&window_id) {
                    app_ctx.send_event(
                        &root_widget,
                        elwt,
                        Event::Internal(InternalEvent::RouteRedrawRequest(target)),
                        &env,
                    )
                } else {
                    tracing::warn!("unregistered window id: {:?}", window_id);
                }
            }
            _ => (),
        }
    })
}
