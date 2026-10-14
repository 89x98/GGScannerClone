use crate::capture::Capturer;
use crate::positioning::Rect;
use anyhow::Result;
use image::buffer::ConvertBuffer;
use image::{RgbImage, RgbaImage};

pub struct ScreenshotsCapturer {
    screens: Vec<screenshots::Screen>,
}

impl ScreenshotsCapturer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            screens: screenshots::Screen::all()?,
        })
    }
}

impl Capturer<RgbaImage> for ScreenshotsCapturer {
    fn capture_rect(&self, rect: Rect<i32>) -> Result<RgbaImage> {
        let screen = &self.screens[0];
        let captured =
            screen.capture_area(rect.left, rect.top, rect.width as u32, rect.height as u32)?;
        let (width, height) = captured.dimensions();
        // screenshots still uses image 0.24; move its RGBA pixels across the
        // crate boundary without changing channel order or copying the buffer.
        Ok(RgbaImage::from_raw(width, height, captured.into_raw())
            .expect("valid screenshot RGBA buffer"))
    }
}

impl Capturer<RgbImage> for ScreenshotsCapturer {
    fn capture_rect(&self, rect: Rect<i32>) -> Result<RgbImage> {
        let rgba_result: RgbaImage = self.capture_rect(rect)?;
        Ok(rgba_result.convert())
    }
}
