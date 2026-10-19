pub use capture_method::CaptureMethod;
pub use capturer::Capturer;

mod capture_method;
mod capturer;
pub mod constants;

// windows

#[cfg(target_os = "windows")]
mod screenshots_capturer;
#[cfg(target_os = "windows")]
mod wgc_capturer;
#[cfg(target_os = "windows")]
mod winapi_capturer;
#[cfg(target_os = "windows")]
mod windows_capturer;

#[cfg(target_os = "windows")]
pub use screenshots_capturer::ScreenshotsCapturer;
#[cfg(target_os = "windows")]
pub use wgc_capturer::WgcCapturer;
#[cfg(target_os = "windows")]
pub use winapi_capturer::WinapiCapturer;
#[cfg(target_os = "windows")]
pub use windows_capturer::WindowsCapturer;

// linux
#[cfg(target_os = "linux")]
mod x11_capturer;

#[cfg(target_os = "linux")]
pub use x11_capturer::X11Capturer;

mod native;
pub use native::create_capturer;
