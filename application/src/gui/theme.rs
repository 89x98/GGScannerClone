use eframe::egui::{self, Color32, RichText, Stroke};

use super::state::{Lang, TaskStatus};

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

pub fn setup(ctx: &egui::Context) {
    // Keep native widgets on the same dark theme as the custom panel backgrounds.
    ctx.set_theme(egui::Theme::Dark);
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = BACKGROUND;
    style.visuals.window_fill = SURFACE;
    style.visuals.faint_bg_color = SURFACE;
    style.visuals.extreme_bg_color = Color32::from_rgb(17, 23, 30);
    style.visuals.override_text_color = Some(Color32::from_rgb(237, 242, 247));
    style.visuals.selection.bg_fill = Color32::from_rgb(38, 61, 53);
    style.visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    style.visuals.hyperlink_color = ACCENT;
    style.visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);
    style.visuals.error_fg_color = ERROR;
    style.visuals.warn_fg_color = WARNING;
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
        widget.bg_fill = SURFACE;
        widget.weak_bg_fill = SURFACE;
        widget.bg_stroke = Stroke::new(1.0, CONTROL_BORDER);
        widget.rounding = egui::Rounding::same(4.0);
    }
    style.visuals.widgets.hovered.bg_fill = HOVER;
    style.visuals.widgets.hovered.weak_bg_fill = HOVER;
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
    style.visuals.widgets.active.rounding = egui::Rounding::same(4.0);
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
    ctx.set_style(style);
}

pub fn primary_action(ui: &mut egui::Ui, enabled: bool, text: &str) -> egui::Response {
    ui.scope(|ui| {
        ui.visuals_mut().widgets.inactive.weak_bg_fill = if enabled { ACCENT } else { SELECTED };
        ui.visuals_mut().widgets.noninteractive.weak_bg_fill = SELECTED;
        ui.visuals_mut().widgets.hovered.weak_bg_fill = Color32::from_rgb(155, 234, 211);
        ui.visuals_mut().widgets.active.weak_bg_fill = Color32::from_rgb(108, 203, 177);
        ui.add_enabled_ui(enabled, |ui| {
            ui.add_sized(
                [ui.available_width(), 36.0],
                egui::Button::new(RichText::new(text).color(if enabled { Color32::from_rgb(16, 41, 31) } else { MUTED }))
                    .stroke(Stroke::NONE)
                    .rounding(5.0),
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
    ui.scope(|ui| {
        let visuals = ui.visuals_mut();
        visuals.override_text_color = None;
        visuals.widgets.inactive.weak_bg_fill = if selected { SELECTED } else { Color32::TRANSPARENT };
        visuals.widgets.inactive.bg_stroke = Stroke::NONE;
        visuals.widgets.inactive.fg_stroke.color = if selected { ACCENT } else { MUTED };
        visuals.widgets.hovered.bg_stroke = Stroke::NONE;
        visuals.widgets.active.bg_stroke = Stroke::NONE;
        visuals.widgets.active.weak_bg_fill = PRESSED;
        visuals.widgets.hovered.fg_stroke.color = ACCENT;
        visuals.widgets.active.fg_stroke.color = ACCENT;
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
                SUCCESS
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
                    SUCCESS
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
