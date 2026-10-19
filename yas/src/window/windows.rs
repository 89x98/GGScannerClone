use super::{FocusRequest, NativeWindow, NativeWindowHandle, Rect, Result};
use crate::utils;
pub(super) fn client_rect(window: NativeWindowHandle) -> Result<Rect<i32>> {
    utils::get_client_rect(window as _)
}
pub(super) fn is_valid(window: NativeWindowHandle) -> bool {
    utils::is_window_handle_valid(window as _)
}
pub(super) fn is_foreground(window: NativeWindowHandle) -> bool {
    unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() as isize == window }
}
pub(super) fn request_focus(
    window: NativeWindowHandle,
    allow_activation: bool,
) -> Result<FocusRequest> {
    if allow_activation {
        unsafe {
            use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                keybd_event, KEYEVENTF_KEYUP, VK_MENU,
            };
            keybd_event(VK_MENU as u8, 0, 0, 0);
            keybd_event(VK_MENU as u8, 0, KEYEVENTF_KEYUP, 0);
        }
    }
    Ok(if utils::show_window_and_set_foreground(window as _)? {
        FocusRequest::Granted
    } else {
        FocusRequest::Denied
    })
}
pub(super) fn enumerate() -> Result<Vec<(NativeWindow, String)>> {
    Ok(utils::iterate_window()
        .into_iter()
        .filter_map(|window| {
            utils::get_window_title(window).map(|title| (NativeWindow(window as isize), title))
        })
        .collect())
}
