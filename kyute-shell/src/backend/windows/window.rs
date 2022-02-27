use windows::Win32::Foundation::HWND;

pub struct WindowBuilder {
    title: String,
}

pub struct Window {
    hwnd: HWND,
}

impl Window {
    pub fn new() -> Window {}
}
