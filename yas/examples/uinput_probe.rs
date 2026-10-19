#[cfg(target_os = "linux")]
mod linux {
    // Diagnostic for the direct-uinput input path (Linux).
    //
    // Creates the scanner's virtual devices, then verifies — against a temporary
    // override-redirect X11 window — that (a) relative pointer motion arrives,
    // (b) touchscreen taps land at exact coordinates.
    //
    // Requires `/dev/uinput` to be writable (input group / 80-uinput.rules).
    //
    // ```text
    // cargo run -p yas_core --example uinput_probe
    // ```

    use std::time::Duration;

    use anyhow::{Context, Result};
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{
        ConfigureWindowAux, ConnectionExt, CreateWindowAux, EventMask, WindowClass,
    };

    use yas_core::system_control::linux::uinput_dev::UinputDevices;

    fn pointer_pos<C: Connection>(conn: &C, root: u32) -> Result<(i32, i32)> {
        let reply = conn.query_pointer(root)?.reply()?;
        Ok((reply.root_x as i32, reply.root_y as i32))
    }

    pub fn run() -> Result<()> {
        let mut devices = UinputDevices::create().context("creating the virtual input devices")?;
        println!(
            "virtual devices created (touchscreen: {})",
            devices.has_touch()
        );

        let (conn, screen) = x11rb::connect(None).context("connecting to X11 (XWayland)")?;
        let root = conn.setup().roots[screen].root;
        let root_w = conn.setup().roots[screen].width_in_pixels as i32;
        let root_h = conn.setup().roots[screen].height_in_pixels as i32;
        println!("X screen: {root_w}x{root_h}");

        // An override-redirect window covering the screen, so the pointer is over
        // an X11 surface: its position is readable and clicks are delivered to
        // something we can observe.
        let win = conn.generate_id()?;
        let aux = CreateWindowAux::new()
            .event_mask(EventMask::BUTTON_PRESS | EventMask::POINTER_MOTION)
            .override_redirect(1)
            .background_pixel(0x103050);
        conn.create_window(
            24,
            win,
            root,
            0,
            0,
            root_w as u16,
            root_h as u16,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &aux,
        )?;
        conn.configure_window(
            win,
            &ConfigureWindowAux::new().stack_mode(x11rb::protocol::xproto::StackMode::ABOVE),
        )?;
        conn.map_window(win)?;
        conn.flush()?;
        std::thread::sleep(Duration::from_millis(400));

        // Bring the pointer to the screen centre with one slam+delta (like the
        // scanner's first move), then measure relative motion.
        devices.pointer_slam()?;
        std::thread::sleep(Duration::from_millis(60));
        devices.pointer_move_rel(root_w / 2, root_h / 2)?;
        std::thread::sleep(Duration::from_millis(300));
        println!("pointer after slam+delta: {:?}", pointer_pos(&conn, root)?);

        for (label, deltas, gap) in [
            ("single +100 (idle)", vec![100], 1.0),
            ("10 x +10 @300ms", vec![10; 10], 0.3),
            ("100 x +1 @5ms", vec![1; 100], 0.005),
        ] {
            let mut p0 = pointer_pos(&conn, root)?;
            // Keep the pointer on screen.
            if p0.0 > root_w - 300 {
                devices.pointer_move_rel(-(p0.0 - root_w / 2), 0)?;
                std::thread::sleep(Duration::from_millis(150));
                p0 = pointer_pos(&conn, root)?;
            }
            for d in &deltas {
                devices.pointer_move_rel(*d, 0)?;
                std::thread::sleep(Duration::from_secs_f64(gap));
            }
            std::thread::sleep(Duration::from_millis(250));
            let p1 = pointer_pos(&conn, root)?;
            let moved = p1.0 - p0.0;
            let total = deltas.iter().sum::<i32>();
            println!(
                "{label:22} sent {total:+5} -> moved {moved:+5}px  factor={:.2}",
                moved as f64 / total as f64
            );
        }

        if devices.has_touch() {
            let (tx, ty) = (root_w / 3, root_h / 2);
            devices.touch_tap(tx, ty, root_w, root_h)?;
            std::thread::sleep(Duration::from_millis(500));
            let mut clicks = Vec::new();
            while let Some(event) = conn.poll_for_event()? {
                if let x11rb::protocol::Event::ButtonPress(press) = event {
                    clicks.push((press.event_x as i32, press.event_y as i32));
                }
            }
            println!(
            "touch tap at ({tx},{ty}) -> ButtonPress events: {clicks:?} (expect one at ~({tx},{ty}))"
        );
        }

        conn.unmap_window(win)?;
        conn.destroy_window(win)?;
        conn.flush()?;
        Ok(())
    }
}
#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    linux::run()
}
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("This diagnostic requires Linux.");
}
