#![cfg(target_os = "linux")]

use x11rb::{
    connection::Connection,
    protocol::{
        xproto::{self, ConnectionExt as _},
        Event,
    },
    wrapper::ConnectionExt as _,
};
use yas_core::{
    cancel::{CancelToken, StopReason},
    capture::{Capturer, X11Capturer},
    positioning::Rect,
    system_control::SystemControl,
    utils,
};

/// Run on an isolated display: xvfb-run -a cargo test -p yas_core --test linux_x11 -- --ignored
#[test]
#[ignore = "requires an isolated X11 display; injects input"]
fn window_capture_input_and_cancellation() -> anyhow::Result<()> {
    let (conn, screen) = x11rb::connect(None)?;
    let root = &conn.setup().roots[screen];
    let window = conn.generate_id()?;
    conn.create_window(
        root.root_depth,
        window,
        root.root,
        40,
        50,
        160,
        90,
        0,
        xproto::WindowClass::INPUT_OUTPUT,
        root.root_visual,
        &xproto::CreateWindowAux::new()
            .background_pixel(0x123456)
            .event_mask(
                xproto::EventMask::BUTTON_PRESS
                    | xproto::EventMask::BUTTON_RELEASE
                    | xproto::EventMask::KEY_PRESS
                    | xproto::EventMask::KEY_RELEASE,
            ),
    )?
    .check()?;
    let name = conn.intern_atom(false, b"_NET_WM_NAME")?.reply()?.atom;
    let utf8 = conn.intern_atom(false, b"UTF8_STRING")?.reply()?.atom;
    conn.change_property8(
        xproto::PropMode::REPLACE,
        window,
        name,
        utf8,
        "原神".as_bytes(),
    )?
    .check()?;
    conn.map_window(window)?.check()?;
    conn.flush()?;
    assert!(utils::enumerate_windows()?
        .iter()
        .any(|w| w.window == window && w.title == "原神"));
    assert_eq!(utils::get_client_rect(window)?, Rect::new(40, 50, 160, 90));
    utils::activate_window(window)?;
    assert!(utils::is_window_foreground(window));

    let capture = X11Capturer::new(window)?;
    let img = capture.capture_rect(Rect::new(38, 47, 164, 96))?;
    assert_eq!(img.dimensions(), (164, 96));
    assert_eq!(img.get_pixel(0, 0).0, [0, 0, 0]);
    assert_eq!(img.get_pixel(2, 3).0, [0x12, 0x34, 0x56]);
    assert_eq!(img.get_pixel(161, 92).0, [0x12, 0x34, 0x56]);
    assert_eq!(img.get_pixel(162, 93).0, [0, 0, 0]);
    assert!(capture.capture_rect(Rect::new(40, 50, -1, 90)).is_err());
    assert!(capture.capture_rect(Rect::new(40, 50, 160, 0)).is_err());

    // Force the XTEST path without requiring access to /dev/uinput.
    std::env::remove_var("WAYLAND_DISPLAY");
    let mut control = SystemControl::new();
    control.prepare()?;
    control.mouse_click_at(80, 80)?;
    control.mouse_scroll(1, false)?;
    control.mouse_scroll_wheel_delta(120)?;
    control.key_press(enigo::Key::Layout('b'))?;
    control.mouse_drag(80, 80, 100, 90)?;
    conn.get_input_focus()?.reply()?; // synchronize the fixture connection
    let mut buttons = Vec::new();
    let mut keys = 0;
    while let Some(event) = conn.poll_for_event()? {
        match event {
            Event::ButtonPress(e) => buttons.push(e.detail),
            Event::KeyPress(_) => keys += 1,
            _ => {},
        }
    }
    assert_eq!(buttons, [1, 5, 4, 1], "click, downward wheel, raw upward wheel, drag");
    assert_eq!(keys, 1);
    let pointer = utils::query_pointer()?;
    assert_eq!((pointer.root_x, pointer.root_y), (100, 90));
    let token = CancelToken::new();
    utils::fake_input(xproto::BUTTON_PRESS_EVENT, 3, 0, 0)?;
    assert!(token.check_rmb());
    assert_eq!(token.reason(), Some(StopReason::UserAbort));
    utils::fake_input(xproto::BUTTON_RELEASE_EVENT, 3, 0, 0)?;
    conn.destroy_window(window)?.check()?;
    assert!(!utils::is_window_handle_valid(window));
    assert!(capture.capture_rect(Rect::new(40, 50, 10, 10)).is_err());
    Ok(())
}
