use keyboard_types::{CompositionEvent, KeyboardEvent, Modifiers};

/// Represents the type of pointer.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PointerType {
    Mouse,
    Pen,
    Stylus,
    Other,
}

/// Represents a pointer button.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct PointerButton(pub u16);

impl PointerButton {
    pub const LEFT: PointerButton = PointerButton(0); // Or touch/pen contact
    pub const MIDDLE: PointerButton = PointerButton(1);
    pub const RIGHT: PointerButton = PointerButton(2); // Or pen barrel
    pub const X1: PointerButton = PointerButton(3);
    pub const X2: PointerButton = PointerButton(4);
}

/// The state of the mouse buttons.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct PointerButtons(pub u32);

impl PointerButtons {
    /// Checks if the specified mouse button is pressed.
    pub fn test(self, button: PointerButton) -> bool {
        self.0 & (1u32 << button.0 as u32) != 0
    }
    pub fn set(&mut self, button: PointerButton) {
        self.0 |= 1u32 << button.0 as u32;
    }
    pub fn reset(&mut self, button: PointerButton) {
        self.0 &= !(1u32 << button.0 as u32);
    }
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

impl Default for PointerButtons {
    fn default() -> Self {
        PointerButtons(0)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PointerEventKind {
    PointerDown,
    PointerUp,
    PointerMove,
    PointerOver,
    PointerOut,
}

/// Modeled after [W3C's PointerEvent](https://www.w3.org/TR/pointerevents3/#pointerevent-interface)
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PointerEvent {
    pub kind: PointerEventKind,
    /// Position in device-independent (logical) pixels, relative to the visual node that the event
    /// is delivered to.
    pub position: Point,
    /// Window position.
    pub window_position: Point,
    /// State of the keyboard modifiers when this event was emitted.
    pub modifiers: Modifiers,
    /// The state of the mouse buttons when this event was emitted.
    pub buttons: PointerButtons,
    /// Identifies the pointer.
    pub pointer_id: winit::event::DeviceId,
    /// The button that triggered this event, if there is one.
    pub button: Option<PointerButton>,
    /// The repeat count for double, triple (and more) for button press events (`Event::PointerDown`).
    /// Otherwise, the value is unspecified.
    pub repeat_count: u32,
    //pub contact_width: f64,
    //pub contact_height: f64,
    //pub pressure: f32,
    //pub tangential_pressure: f32,
    //pub tilt_x: i32,
    //pub tilt_y: i32,
    //pub twist: i32,
    //pub pointer_type: PointerType,
    //pub primary: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum WheelDeltaMode {
    Pixel,
    Line,
    Page,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct WheelEvent {
    pub pointer: PointerEvent,
    pub delta_x: f64,
    pub delta_y: f64,
    pub delta_z: f64,
    pub delta_mode: WheelDeltaMode,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct InputEvent {
    pub character: char,
}

//--------------------------------------------------------------------------------------------------

pub enum WindowEvent {
    PointerEvent(PointerEvent),
    Wheel(WheelEvent),
    Keyboard(KeyboardEvent),
    Composition(CompositionEvent),
    Resized(SizeI),
    RedrawRequested,
}

pub struct Event {}
