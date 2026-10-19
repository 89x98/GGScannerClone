use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::null_mut;

use anyhow::{anyhow, Result};
use image::{ImageBuffer, RgbImage};
use windows_sys::Win32::Graphics::Gdi::*;

use crate::capture::Capturer;
use crate::positioning::{Pos, Rect};

struct ScreenDc(HDC);

impl Drop for ScreenDc {
    fn drop(&mut self) {
        unsafe { ReleaseDC(null_mut(), self.0) };
    }
}

/// Owns both resources even when allocation or a later capture step fails.
struct CaptureBitmap {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
}

impl CaptureBitmap {
    unsafe fn new(source: HDC, width: i32, height: i32) -> Result<Self> {
        let mut capture = Self {
            dc: CreateCompatibleDC(source),
            bitmap: null_mut(),
            previous: null_mut(),
        };
        if capture.dc.is_null() {
            return Err(anyhow!("CreateCompatibleDC failed"));
        }
        capture.bitmap = CreateCompatibleBitmap(source, width, height);
        if capture.bitmap.is_null() {
            return Err(anyhow!("CreateCompatibleBitmap failed"));
        }
        capture.previous = SelectObject(capture.dc, capture.bitmap);
        if capture.previous.is_null() {
            return Err(anyhow!("SelectObject failed"));
        }
        Ok(capture)
    }

    unsafe fn deselect(&mut self) -> Result<()> {
        if SelectObject(self.dc, self.previous).is_null() {
            return Err(anyhow!(
                "SelectObject failed to restore the original bitmap"
            ));
        }
        self.previous = null_mut();
        Ok(())
    }
}

impl Drop for CaptureBitmap {
    fn drop(&mut self) {
        unsafe {
            if !self.previous.is_null() {
                SelectObject(self.dc, self.previous);
            }
            // Delete the owning DC first so even a failed restoration cannot
            // leave our bitmap selected when it is deleted.
            if !self.dc.is_null() {
                DeleteDC(self.dc);
            }
            if !self.bitmap.is_null() {
                DeleteObject(self.bitmap);
            }
        }
    }
}

fn capture_buffer_size(rect: Rect<i32>) -> Result<usize> {
    if rect.width <= 0 || rect.height <= 0 {
        return Err(anyhow!("Capture dimensions must be positive"));
    }
    (rect.width as usize)
        .checked_mul(rect.height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .filter(|bytes| *bytes <= isize::MAX as usize)
        .ok_or_else(|| anyhow!("Capture dimensions exceed the buffer size limit"))
}

// Bottom-up BGRA, preserving the existing pixel format and screen coordinates.
unsafe fn capture_from_dc(source: HDC, rect: Rect<i32>) -> Result<Vec<u8>> {
    let bitmap_size = capture_buffer_size(rect)?;
    let mut capture = CaptureBitmap::new(source, rect.width, rect.height)?;
    let result = BitBlt(
        capture.dc,
        0,
        0,
        rect.width,
        rect.height,
        source,
        rect.left,
        rect.top,
        SRCCOPY,
    );
    if result == 0 {
        return Err(anyhow!("BitBlt failed"));
    }

    let mut bitmap: BITMAP = std::mem::zeroed();
    if GetObjectW(
        capture.bitmap,
        size_of::<BITMAP>() as i32,
        (&mut bitmap) as *mut BITMAP as *mut c_void,
    ) != size_of::<BITMAP>() as i32
        || bitmap.bmWidth != rect.width
        || bitmap.bmHeight != rect.height
    {
        return Err(anyhow!(
            "GetObjectW returned invalid capture bitmap dimensions"
        ));
    }

    let mut info: BITMAPINFO = std::mem::zeroed();
    info.bmiHeader = BITMAPINFOHEADER {
        biSize: size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: bitmap.bmWidth,
        biHeight: bitmap.bmHeight,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        biSizeImage: 0,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    };

    let mut buffer: Vec<u8> = vec![0; bitmap_size];

    // GetDIBits requires the bitmap to be deselected from its memory DC.
    capture.deselect()?;
    let lines = GetDIBits(
        source,
        capture.bitmap,
        0,
        bitmap.bmHeight as u32,
        buffer.as_mut_ptr() as *mut c_void,
        &mut info,
        DIB_RGB_COLORS,
    );

    if lines != bitmap.bmHeight {
        return Err(anyhow!(
            "GetDIBits copied {lines} of {} capture rows",
            bitmap.bmHeight
        ));
    }

    Ok(buffer)
}

unsafe fn unsafe_capture(rect: Rect<i32>) -> Result<Vec<u8>> {
    capture_buffer_size(rect)?;
    let screen = ScreenDc(GetDC(null_mut()));
    if screen.0.is_null() {
        return Err(anyhow!("GetDC failed"));
    }
    capture_from_dc(screen.0, rect)
}

pub struct WinapiCapturer;

impl WinapiCapturer {
    pub fn new() -> Self {
        // todo maybe we can explicitly account for windows scale, and remove the call
        // crate::utils::set_dpi_awareness();
        Self
    }
}

impl Capturer<RgbImage> for WinapiCapturer {
    fn capture_rect(&self, rect: Rect<i32>) -> Result<RgbImage> {
        let raw: Vec<u8> = unsafe { unsafe_capture(rect)? };

        let height = rect.height as u32;
        let width = rect.width as u32;

        let img = ImageBuffer::from_fn(rect.width as u32, rect.height as u32, move |x, y| {
            let y = height - y - 1;
            let b = raw[((y * width + x) * 4 + 0) as usize];
            let g = raw[((y * width + x) * 4 + 1) as usize];
            let r = raw[((y * width + x) * 4 + 2) as usize];
            image::Rgb([r, g, b])
        });

        Ok(img)
    }

    fn capture_color(&self, pos: Pos<i32>) -> Result<image::Rgb<u8>> {
        let raw: Vec<u8> = unsafe {
            unsafe_capture(Rect {
                left: pos.x,
                top: pos.y,
                width: 1,
                height: 1,
            })?
        };
        let r = raw[2];
        let g = raw[1];
        let b = raw[0];
        Ok(image::Rgb([r, g, b]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS,
    };

    fn gdi_objects() -> u32 {
        unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) }
    }

    #[test]
    fn capture_preserves_pixels_and_releases_resources_on_success_and_error() -> Result<()> {
        unsafe {
            let screen = ScreenDc(GetDC(null_mut()));
            assert!(!screen.0.is_null());
            let source = CaptureBitmap::new(screen.0, 3, 2)?;
            // COLORREF uses 0x00BBGGRR. Distinct rows and columns catch a
            // changed crop origin, channel order, or vertical orientation.
            let colors = [
                [0x000000ff, 0x0000ff00, 0x00ff0000],
                [0x00332211, 0x00665544, 0x00998877],
            ];
            for (y, row) in colors.iter().enumerate() {
                for (x, color) in row.iter().enumerate() {
                    assert_ne!(SetPixelV(source.dc, x as i32, y as i32, *color), 0);
                }
            }
            let rect = Rect {
                left: 1,
                top: 0,
                width: 2,
                height: 2,
            };
            let expected = [
                [0x66, 0x55, 0x44],
                [0x99, 0x88, 0x77],
                [0x00, 0xff, 0x00],
                [0xff, 0x00, 0x00],
            ];
            let check_pixels = |raw: &[u8]| {
                assert_eq!(raw.len(), 16);
                // The unused fourth byte is not part of the RGB contract.
                for (pixel, expected) in raw.chunks_exact(4).zip(expected) {
                    assert_eq!(&pixel[..3], expected);
                }
            };
            check_pixels(&capture_from_dc(source.dc, rect)?);
            // Warm up the public screen path before measuring GDI objects.
            let capturer = WinapiCapturer::new();
            capturer.capture_color(Pos { x: 0, y: 0 })?;
            let before = gdi_objects();
            assert!(before > 0);
            for _ in 0..128 {
                check_pixels(&capture_from_dc(source.dc, rect)?);
                capturer.capture_rect(Rect {
                    left: 0,
                    top: 0,
                    width: 2,
                    height: 2,
                })?;
                capturer.capture_color(Pos { x: 0, y: 0 })?;
                let failed: Result<()> = (|| {
                    let _partial = CaptureBitmap::new(screen.0, 3, 2)?;
                    // Exercise early-return cleanup while the bitmap is still
                    // selected, as on a failed BitBlt or GetObjectW call.
                    Err(anyhow!("capture failed after allocation"))
                })();
                assert!(failed.is_err());
            }
            assert_eq!(gdi_objects(), before, "capture leaked GDI objects");
            for (width, height) in [(0, 2), (2, 0), (-1, 2), (2, -1)] {
                assert!(unsafe_capture(Rect {
                    left: 0,
                    top: 0,
                    width,
                    height
                })
                .is_err());
            }
            assert_eq!(gdi_objects(), before);
        }
        Ok(())
    }
}
