//! User interface component.
//!
//! ui/common: common GUI stuff (window, etc.). Should move into another crate eventually
//! ui/application: application related GUI (document windows, etc.)
//! ui/common/widgets: reusable widgets
//! ui/common/platform: platform-specific code
//! ui/common/model: data model stuff
//!

pub mod common;
pub mod document;
pub mod kyute;


pub trait Application {
    type Action: 'static;

    fn update(&mut self);

    fn view(&mut self) -> kyute::Widget<>
}