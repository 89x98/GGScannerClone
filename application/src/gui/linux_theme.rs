use std::sync::mpsc::Receiver;

use eframe::egui;

use egui::Theme;

/// Portal identifiers for the appearance color-scheme setting.
const PORTAL_DEST: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
const PORTAL_IFACE: &str = "org.freedesktop.portal.Settings";
const APPEARANCE_NS: &str = "org.freedesktop.appearance";
const COLOR_SCHEME_KEY: &str = "color-scheme";

/// Spawn the detection thread; each resolved theme is sent through the
/// channel (initial value first, then live desktop switches). Returns `None`
/// on platforms where winit already follows the system theme.
#[cfg(target_os = "linux")]
pub fn spawn_theme_watcher() -> Option<Receiver<Option<Theme>>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("theme-watcher".to_owned())
        .spawn(move || watch_desktop_theme(&tx))
        .ok()?;
    Some(rx)
}

#[cfg(target_os = "linux")]
fn watch_desktop_theme(tx: &std::sync::mpsc::Sender<Option<Theme>>) {
    let send = |theme: Option<Theme>| {
        let _ = tx.send(theme);
    };

    let conn = match zbus::blocking::Connection::session() {
        Ok(conn) => conn,
        Err(e) => {
            yas::log_debug!(
                "无法连接 D-Bus 会话总线，主题检测回退 gsettings: {e}",
                "D-Bus session bus unavailable; falling back to gsettings for theme detection: {e}",
            );
            send(detect_via_gsettings());
            return;
        },
    };

    // Initial value.
    send(read_portal_color_scheme(&conn));

    // Live changes.
    let proxy = zbus::blocking::Proxy::new(&conn, PORTAL_DEST, PORTAL_PATH, PORTAL_IFACE);
    match proxy.and_then(|proxy| proxy.receive_signal("SettingChanged")) {
        Ok(mut signals) => {
            while let Some(message) = signals.next() {
                let body = message.body();
                let Ok((namespace, key, value)) =
                    body.deserialize::<(String, String, zbus::zvariant::Value)>()
                else {
                    continue;
                };
                if namespace == APPEARANCE_NS && key == COLOR_SCHEME_KEY {
                    send(color_scheme_value_to_theme(&value));
                }
            }
        },
        Err(e) => {
            // No portal (minimal WMs): the initial detection already ran via
            // the portal read's own fallback.
            yas::log_debug!(
                "无法订阅主题变更信号: {e}",
                "Cannot subscribe to theme-change signals: {e}",
            );
        },
    }
}

#[cfg(target_os = "linux")]
fn read_portal_color_scheme(conn: &zbus::blocking::Connection) -> Option<Theme> {
    let reply = conn.call_method(
        Some(PORTAL_DEST),
        PORTAL_PATH,
        Some(PORTAL_IFACE),
        "ReadOne",
        &(APPEARANCE_NS, COLOR_SCHEME_KEY),
    );
    match reply {
        Ok(reply) => match reply.body().deserialize::<zbus::zvariant::Value>() {
            Ok(value) => color_scheme_value_to_theme(&value),
            Err(_) => None,
        },
        Err(_) => detect_via_gsettings(),
    }
}

/// xdg-desktop-portal color-scheme values: 0 = no preference, 1 = prefer
/// dark, 2 = prefer light.
#[cfg(target_os = "linux")]
fn color_scheme_value_to_theme(value: &zbus::zvariant::Value<'_>) -> Option<Theme> {
    match value {
        zbus::zvariant::Value::U32(1) => Some(Theme::Dark),
        zbus::zvariant::Value::U32(2) => Some(Theme::Light),
        _ => None,
    }
}

/// Fallback for sessions without a portal: `gsettings get
/// org.gnome.desktop.interface color-scheme` (works on GTK-based desktops
/// and on KDE when glib tools are installed).
#[cfg(target_os = "linux")]
fn detect_via_gsettings() -> Option<Theme> {
    let output = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
        .ok()?;
    let value = String::from_utf8_lossy(&output.stdout);
    if value.contains("prefer-dark") {
        Some(Theme::Dark)
    } else if value.contains("prefer-light") {
        Some(Theme::Light)
    } else {
        None
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn portal_values_include_reset_to_system() {
        assert_eq!(
            color_scheme_value_to_theme(&zbus::zvariant::Value::U32(1)),
            Some(Theme::Dark)
        );
        assert_eq!(
            color_scheme_value_to_theme(&zbus::zvariant::Value::U32(2)),
            Some(Theme::Light)
        );
        assert_eq!(
            color_scheme_value_to_theme(&zbus::zvariant::Value::U32(0)),
            None
        );
        assert_eq!(
            color_scheme_value_to_theme(&zbus::zvariant::Value::U32(9)),
            None
        );
    }
}

use super::theme::preference;
/// Resolve the theme to apply, given the configured appearance and (on Linux,
/// where winit reports no system theme) what the desktop actually reports.
///
/// An explicit Light/Dark choice always wins; `System` prefers the detected
/// desktop theme and falls back to `ThemePreference::System`, which lets winit
/// decide on platforms that report it themselves.
pub fn effective_preference(
    appearance: crate::config::Appearance,
    desktop_theme: Option<egui::Theme>,
) -> egui::ThemePreference {
    match (appearance, desktop_theme) {
        (crate::config::Appearance::System, Some(theme)) => match theme {
            egui::Theme::Light => egui::ThemePreference::Light,
            egui::Theme::Dark => egui::ThemePreference::Dark,
        },
        _ => preference(appearance),
    }
}
