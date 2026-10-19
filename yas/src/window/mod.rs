//! Non-owning native window adapter. Only backend modules call OS window APIs.
//! Game title/class selection, navigation, recognition and retry policy stay in
//! game modules and must not be duplicated per operating system.
use crate::positioning::Rect;
#[cfg(target_os = "windows")]
pub type NativeWindowHandle = isize;
#[cfg(target_os = "linux")]
pub type NativeWindowHandle = u32;
use anyhow::Result;
#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod backend;
#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod backend;

/// Activation can be synchronous (Win32) or compositor-mediated (XWayland).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusRequest {
    Granted,
    Denied,
    Pending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeWindow(pub NativeWindowHandle);

impl NativeWindow {
    pub fn client_rect(self) -> Result<Rect<i32>> {
        backend::client_rect(self.0)
    }
    pub fn is_valid(self) -> bool {
        backend::is_valid(self.0)
    }
    pub fn is_foreground(self) -> bool {
        backend::is_foreground(self.0)
    }
    /// `allow_activation` permits the existing Windows Alt-key activation aid.
    /// A pending request requires a later foreground check; it is never success.
    pub fn request_focus(self, allow_activation: bool) -> Result<FocusRequest> {
        backend::request_focus(self.0, allow_activation)
    }
}

pub fn enumerate() -> Result<Vec<(NativeWindow, String)>> {
    backend::enumerate()
}
