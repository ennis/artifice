//! Platform-specific application code.
//!
//! Currently, there is only an implementation for windows.
pub mod windows;

pub use windows::PlatformWindow;
pub use windows::Platform;
pub use windows::PaintCtx;