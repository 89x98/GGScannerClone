//! OS selection lives here; game controllers share capture policy and OCR.
use super::{CaptureMethod, Capturer};
use anyhow::Result;
use image::RgbImage;
use std::rc::Rc;

use crate::window::NativeWindowHandle;

/// Create the native window capturer. Windows keeps BitBlt/WGC selection;
/// Linux uses X11/XWayland and rejects unsupported HDR before capturing.
pub fn create_capturer(
    window: NativeWindowHandle,
    method: CaptureMethod,
    hdr: bool,
    white_point: f32,
) -> Result<Rc<dyn Capturer<RgbImage>>> {
    #[cfg(target_os = "windows")]
    {
        match method {
            CaptureMethod::BitBlt => Ok(Rc::new(super::WindowsCapturer::new()?)),
            CaptureMethod::Wgc => Ok(Rc::new(super::WgcCapturer::new(window, hdr, white_point)?)),
        }
    }
    #[cfg(target_os = "linux")]
    {
        let _ = (method, white_point);
        anyhow::ensure!(
            !hdr,
            "Linux capture requires SDR; turn off HDR in the game and scanner settings"
        );
        Ok(Rc::new(super::X11Capturer::new(window)?))
    }
}
