//! Desktop theme plumbing: palette, style installation, and Linux detection.
//!
//! The palette/styles come from the shared UI design (light and dark). Where the
//! theme choice itself comes from depends on the platform:
//!
//! - Windows/macOS: winit reports the system theme, so `ThemePreference::System`
//!   is enough and no watcher is needed.
//! - Linux: winit cannot detect the desktop theme (it reports `None`), so egui
//!   always falls back to dark. [`spawn_theme_watcher`] reads the standard
//!   `org.freedesktop.appearance.color-scheme` setting from xdg-desktop-portal
//!   (supported by KDE, GNOME and most compositors) and forwards both the
//!   initial value and live changes via the portal's `SettingChanged` signal.
//!   When no portal is running, `gsettings` is probed once as a fallback.

use std::sync::mpsc::Receiver;

use eframe::egui::{self, Color32, RichText, Stroke};

use super::state::{Lang, TaskStatus};
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
pub fn spawn_theme_watcher() -> Option<Receiver<Theme>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("theme-watcher".to_owned())
        .spawn(move || watch_desktop_theme(&tx))
        .ok()?;
    Some(rx)
}

#[cfg(not(target_os = "linux"))]
pub fn spawn_theme_watcher() -> Option<Receiver<Theme>> {
    None
}

#[cfg(target_os = "linux")]
fn watch_desktop_theme(tx: &std::sync::mpsc::Sender<Theme>) {
    let send = |theme: Option<Theme>| {
        if let Some(theme) = theme {
            let _ = tx.send(theme);
        }
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

    /// Must never panic; asserts a concrete theme only when the environment
    /// can actually answer (CI images may have no portal/gsettings).
    #[test]
    fn theme_detection_does_not_panic() {
        let theme = zbus::blocking::Connection::session()
            .ok()
            .map(|conn| read_portal_color_scheme(&conn));
        if let Some(Some(theme)) = theme {
            // Cross-check: on this machine the portal and gsettings should
            // agree when both can answer.
            if let Some(fallback) = detect_via_gsettings() {
                assert_eq!(fallback, theme, "portal and gsettings disagree");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Palette and widget styling (shared by every platform).
// ---------------------------------------------------------------------------

pub const BACKGROUND: Color32 = Color32::from_rgb(19, 23, 28);
pub const SURFACE: Color32 = Color32::from_rgb(28, 33, 40);
pub const BORDER: Color32 = Color32::from_rgb(51, 61, 73);
pub const ACCENT: Color32 = Color32::from_rgb(124, 220, 192);
pub const SUCCESS: Color32 = Color32::from_rgb(158, 204, 155);
pub const CONTROL_BORDER: Color32 = Color32::from_rgb(103, 119, 137);
pub const HOVER: Color32 = Color32::from_rgb(42, 51, 61);
pub const PRESSED: Color32 = Color32::from_rgb(52, 65, 78);
pub const MUTED: Color32 = Color32::from_rgb(165, 177, 192);
pub const SELECTED: Color32 = Color32::from_rgb(35, 55, 51);
pub const ERROR: Color32 = Color32::from_rgb(244, 132, 137);
pub const WARNING: Color32 = Color32::from_rgb(238, 190, 110);

#[derive(Clone, Copy)]
pub struct Palette {
    pub background: Color32,
    pub surface: Color32,
    pub border: Color32,
    pub accent: Color32,
    pub success: Color32,
    pub control_border: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub muted: Color32,
    pub selected: Color32,
    pub error: Color32,
    pub warning: Color32,
    pub foreground: Color32,
    pub input: Color32,
    pub primary_text: Color32,
    pub primary_hover: Color32,
    pub primary_pressed: Color32,
}

pub fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            background: BACKGROUND,
            surface: SURFACE,
            border: BORDER,
            accent: ACCENT,
            success: SUCCESS,
            control_border: CONTROL_BORDER,
            hover: HOVER,
            pressed: PRESSED,
            muted: MUTED,
            selected: SELECTED,
            error: ERROR,
            warning: WARNING,
            foreground: Color32::from_rgb(237, 242, 247),
            input: Color32::from_rgb(17, 23, 30),
            primary_text: Color32::from_rgb(16, 41, 31),
            primary_hover: Color32::from_rgb(155, 234, 211),
            primary_pressed: Color32::from_rgb(108, 203, 177),
        }
    } else {
        Palette {
            background: Color32::from_rgb(243, 246, 249),
            surface: Color32::WHITE,
            border: Color32::from_rgb(210, 219, 226),
            accent: Color32::from_rgb(18, 112, 89),
            success: Color32::from_rgb(47, 117, 57),
            control_border: Color32::from_rgb(137, 154, 169),
            hover: Color32::from_rgb(230, 238, 243),
            pressed: Color32::from_rgb(216, 227, 235),
            muted: Color32::from_rgb(88, 107, 123),
            selected: Color32::from_rgb(219, 239, 230),
            error: Color32::from_rgb(180, 44, 53),
            warning: Color32::from_rgb(143, 90, 7),
            foreground: Color32::from_rgb(30, 43, 55),
            input: Color32::from_rgb(248, 250, 252),
            primary_text: Color32::WHITE,
            primary_hover: Color32::from_rgb(22, 130, 103),
            primary_pressed: Color32::from_rgb(13, 91, 72),
        }
    }
}

pub fn preference(appearance: crate::config::Appearance) -> egui::ThemePreference {
    match appearance {
        crate::config::Appearance::System => egui::ThemePreference::System,
        crate::config::Appearance::Light => egui::ThemePreference::Light,
        crate::config::Appearance::Dark => egui::ThemePreference::Dark,
    }
}

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

pub fn setup(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    super::icons::add_to_fonts(&mut fonts);
    ctx.set_fonts(fonts);
    install(ctx, crate::config::Appearance::Dark);
}

pub fn install(ctx: &egui::Context, appearance: crate::config::Appearance) {
    for mode in [egui::Theme::Light, egui::Theme::Dark] {
        ctx.set_style_of(
            mode,
            styled((*ctx.style_of(mode)).clone(), mode == egui::Theme::Dark),
        );
    }
    ctx.set_theme(preference(appearance));
}

fn styled(mut style: egui::Style, dark: bool) -> egui::Style {
    let p = palette(dark);
    // Native widgets and custom surfaces share the selected palette.
    style.visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    style.visuals.panel_fill = p.background;
    style.visuals.window_fill = p.surface;
    style.visuals.faint_bg_color = p.surface;
    style.visuals.extreme_bg_color = p.input;
    style.visuals.override_text_color = Some(p.foreground);
    style.visuals.selection.bg_fill = p.selected;
    style.visuals.selection.stroke = Stroke::new(1.0, p.accent);
    style.visuals.hyperlink_color = p.accent;
    style.visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);
    style.visuals.error_fg_color = p.error;
    style.visuals.warn_fg_color = p.warning;
    // Panes have independent clipping. Keep every interaction state within the
    // widget's allocated bounds instead of egui's default 1px hover/press growth.
    for widget in [
        &mut style.visuals.widgets.noninteractive,
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        widget.expansion = 0.0;
    }
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.noninteractive,
    ] {
        widget.bg_fill = p.surface;
        widget.weak_bg_fill = p.surface;
        widget.bg_stroke = Stroke::new(1.0, p.control_border);
        widget.corner_radius = egui::CornerRadius::same(4);
    }
    style.visuals.widgets.hovered.bg_fill = p.hover;
    style.visuals.widgets.hovered.weak_bg_fill = p.hover;
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent);
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(4);
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(4);
    style.spacing.item_spacing = egui::vec2(8.0, 5.0);
    style.spacing.button_padding = egui::vec2(9.0, 4.0);
    style.spacing.interact_size.y = 26.0;
    style.spacing.scroll.bar_width = 6.0;
    style.spacing.scroll.floating = false;
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(23.0));
    style
}

pub fn primary_action(ui: &mut egui::Ui, enabled: bool, text: &str) -> egui::Response {
    ui.scope(|ui| {
        ui.visuals_mut().widgets.inactive.weak_bg_fill = if enabled {
            palette(ui.visuals().dark_mode).accent
        } else {
            palette(ui.visuals().dark_mode).selected
        };
        ui.visuals_mut().widgets.noninteractive.weak_bg_fill =
            palette(ui.visuals().dark_mode).selected;
        ui.visuals_mut().widgets.hovered.weak_bg_fill =
            palette(ui.visuals().dark_mode).primary_hover;
        ui.visuals_mut().widgets.active.weak_bg_fill =
            palette(ui.visuals().dark_mode).primary_pressed;
        ui.add_enabled_ui(enabled, |ui| {
            ui.add_sized(
                [ui.available_width(), 36.0],
                egui::Button::new(RichText::new(text).color(if enabled {
                    palette(ui.visuals().dark_mode).primary_text
                } else {
                    palette(ui.visuals().dark_mode).muted
                }))
                .stroke(Stroke::NONE)
                .corner_radius(5.0),
            )
        })
        .inner
    })
    .inner
}

pub fn secondary_action(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add_sized([ui.available_width(), 30.0], egui::Button::new(text))
}

/// A quiet native button with a visible hover fill, border, and brighter text.
pub fn text_button(
    ui: &mut egui::Ui,
    text: &str,
    size: egui::Vec2,
    selected: bool,
) -> egui::Response {
    let p = palette(ui.visuals().dark_mode);
    ui.scope(|ui| {
        let visuals = ui.visuals_mut();
        visuals.override_text_color = None;
        visuals.widgets.inactive.weak_bg_fill = if selected {
            p.selected
        } else {
            Color32::TRANSPARENT
        };
        visuals.widgets.inactive.bg_stroke = Stroke::NONE;
        visuals.widgets.inactive.fg_stroke.color = if selected { p.accent } else { p.muted };
        visuals.widgets.hovered.bg_stroke = Stroke::NONE;
        visuals.widgets.active.bg_stroke = Stroke::NONE;
        visuals.widgets.active.weak_bg_fill = p.pressed;
        visuals.widgets.hovered.fg_stroke.color = p.accent;
        visuals.widgets.active.fg_stroke.color = p.accent;
        ui.add_sized(size, egui::Button::new(text))
    })
    .inner
}

pub fn task_status(ui: &mut egui::Ui, lang: Lang, status: Option<&TaskStatus>, idle: &str) {
    match status {
        Some(TaskStatus::Running(message)) => {
            ui.add(egui::Label::new(RichText::new(message.text(lang)).size(16.0).strong()).wrap())
                .on_hover_text(message.text(lang));
        },
        Some(TaskStatus::Completed(message))
        | Some(TaskStatus::Stopped(message))
        | Some(TaskStatus::AwaitingInput(message)) => {
            let color = if matches!(
                status,
                Some(TaskStatus::Stopped(_) | TaskStatus::AwaitingInput(_))
            ) {
                ui.visuals().warn_fg_color
            } else {
                palette(ui.visuals().dark_mode).success
            };
            ui.add(
                egui::Label::new(
                    RichText::new(message.text(lang))
                        .size(16.0)
                        .strong()
                        .color(color),
                )
                .wrap(),
            );
        },
        Some(TaskStatus::Exported { path, partial, .. }) => {
            let title = if *partial {
                lang.t("部分结果已导出", "Partial results saved")
            } else {
                lang.t("数据已导出", "Export saved")
            };
            ui.add(
                egui::Label::new(RichText::new(title).size(16.0).strong().color(if *partial {
                    ui.visuals().warn_fg_color
                } else {
                    palette(ui.visuals().dark_mode).success
                }))
                .wrap(),
            );
            super::widgets::export_location(ui, path);
        },
        Some(TaskStatus::Failed(error)) => {
            ui.label(
                RichText::new(lang.t("任务未完成", "Task did not finish"))
                    .size(16.0)
                    .strong()
                    .color(ui.visuals().error_fg_color),
            );
            super::widgets::error_card(ui, lang, error);
        },
        _ => {
            ui.add(egui::Label::new(RichText::new(idle).size(16.0).strong()).wrap())
                .on_hover_text(idle);
        },
    }
    ui.add_space(6.0);
}

pub fn blocked_status(ui: &mut egui::Ui, lang: Lang, reason: &str) {
    task_status(
        ui,
        lang,
        Some(&TaskStatus::AwaitingInput(super::state::UiText::new(
            reason, reason,
        ))),
        "",
    );
}

pub fn restart_required(ui: &mut egui::Ui, lang: Lang, error: Option<&super::state::UiError>) {
    super::widgets::status_content(ui, "restart-status", 0, |ui| {
        if let Some(error) = error {
            super::widgets::error_card(ui, lang, error);
        } else {
            ui.colored_label(
                ui.visuals().error_fg_color,
                lang.t(
                    "任务发生内部错误，请重启程序。",
                    "An internal task error requires restarting the app.",
                ),
            );
        }
    });
    primary_action(ui, false, lang.t("需重启程序", "Restart required"));
}
