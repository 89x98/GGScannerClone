use crate::capture::Capturer;
use crate::positioning::Rect;
use crate::utils;
use anyhow::Result;
use image::RgbImage;

/// Captures the game window's own X pixmap via GetImage (ZPixmap).
///
/// Regions use root coordinates and are clipped to the client window. The
/// window must remain mapped, in front and unobscured: core GetImage does not
/// guarantee useful pixels for obscured regions on every X server/compositor.
pub struct X11Capturer {
    window: u32,
}

impl X11Capturer {
    pub fn new(window: u32) -> Result<Self> {
        // Fail early (bad window id / no X connection) instead of on the
        // first capture deep inside a scan.
        let _ = utils::get_client_rect(window)?;
        Ok(Self { window })
    }
}

impl Capturer<RgbImage> for X11Capturer {
    fn capture_rect(&self, rect: Rect<i32>) -> Result<RgbImage> {
        utils::capture_window_region(self.window, rect)
    }
}
