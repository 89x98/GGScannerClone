#![cfg(target_os = "linux")]

use hsr_scanner::device::{HsrDevice, InputCommand, WindowsHsrDevice};
use x11rb::{
    connection::Connection,
    protocol::xproto::{self, ConnectionExt as _},
    wrapper::ConnectionExt as _,
};
use yas::{cancel::CancelToken, capture::CaptureMethod};

#[test]
#[ignore = "requires isolated X11; injects input"]
fn hsr_device_uses_real_linux_capture_and_rejects_geometry_drift() {
    let (conn, screen) = x11rb::connect(None).unwrap();
    let root = &conn.setup().roots[screen];
    let window = conn.generate_id().unwrap();
    conn.create_window(
        root.root_depth,
        window,
        root.root,
        30,
        40,
        960,
        540,
        0,
        xproto::WindowClass::INPUT_OUTPUT,
        root.root_visual,
        &xproto::CreateWindowAux::new().background_pixel(0x123456),
    )
    .unwrap()
    .check()
    .unwrap();
    conn.change_property8(
        xproto::PropMode::REPLACE,
        window,
        xproto::AtomEnum::WM_NAME,
        xproto::AtomEnum::STRING,
        b"Honkai: Star Rail",
    )
    .unwrap()
    .check()
    .unwrap();
    conn.map_window(window).unwrap().check().unwrap();
    conn.flush().unwrap();
    std::env::remove_var("WAYLAND_DISPLAY");
    let cancel = CancelToken::new();
    let mut device =
        WindowsHsrDevice::locate_with_cancel(CaptureMethod::BitBlt, false, cancel.clone()).unwrap();
    device.focus_and_verify().unwrap();
    assert!(device.is_foreground());
    assert_eq!(device.capture_client().unwrap().dimensions(), (960, 540));
    device.input(InputCommand::Key('b')).unwrap();
    conn.configure_window(window, &xproto::ConfigureWindowAux::new().x(40))
        .unwrap()
        .check()
        .unwrap();
    assert!(
        device.capture_client().is_err(),
        "same-size move must invalidate captured geometry"
    );
    device.focus_and_verify().unwrap();
    assert_eq!(device.capture_client().unwrap().dimensions(), (960, 540));
    cancel.cancel(yas::cancel::StopReason::UserAbort);
    assert!(device.input(InputCommand::Key('b')).is_err());
    conn.destroy_window(window).unwrap().check().unwrap();
    assert!(device.capture_client().is_err());
}
