use super::{FocusRequest, NativeWindow, NativeWindowHandle, Rect, Result};
use crate::utils;
pub(super) fn client_rect(window: NativeWindowHandle) -> Result<Rect<i32>> {
    utils::get_client_rect(window)
}
pub(super) fn is_valid(window: NativeWindowHandle) -> bool {
    utils::is_window_handle_valid(window)
}
pub(super) fn is_foreground(window: NativeWindowHandle) -> bool {
    utils::is_window_foreground(window)
}
pub(super) fn request_focus(window: NativeWindowHandle, _: bool) -> Result<FocusRequest> {
    utils::activate_window(window)?;
    Ok(FocusRequest::Pending)
}
pub(super) fn enumerate() -> Result<Vec<(NativeWindow, String)>> {
    Ok(utils::enumerate_windows()?
        .into_iter()
        .map(|w| (NativeWindow(w.window), w.title))
        .collect())
}
