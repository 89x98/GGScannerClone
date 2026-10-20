use crate::game_info::ui::Platform;
use crate::game_info::UI;
use crate::positioning::Rect;

#[derive(Clone, Debug)]
pub struct GameInfo {
    pub window: Rect<i32>,
    pub is_cloud: bool,
    pub ui: UI,
    pub platform: Platform,
    /// Native window handle (HWND on Windows). Used by WGC capturer.
    #[cfg(target_os = "windows")]
    pub hwnd: isize,
    /// X11 window id. Used by the X11 capturer, focus control and liveness
    /// checks (the counterpart of `hwnd` above).
    #[cfg(target_os = "linux")]
    pub window_id: u32,
}

impl GameInfo {
    pub fn native_window(&self) -> crate::window::NativeWindow {
        crate::window::NativeWindow(self.native_window_handle())
    }
    pub fn set_native_window(&mut self, window: crate::window::NativeWindow) {
        #[cfg(target_os = "windows")]
        {
            self.hwnd = window.0;
        }
        #[cfg(target_os = "linux")]
        {
            self.window_id = window.0;
        }
    }

    pub fn native_window_handle(&self) -> crate::window::NativeWindowHandle {
        #[cfg(target_os = "windows")]
        {
            self.hwnd
        }
        #[cfg(target_os = "linux")]
        {
            self.window_id
        }
    }
}
