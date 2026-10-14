use eframe::egui;

use super::theme;

#[derive(Clone, Copy)]
pub enum Pane {
    Settings,
    Status,
}

/// One inset for window edges and both sides of every divider (logical pixels).
pub const PANE_GAP: f32 = 12.0;

pub fn workspace_bounds(rect: egui::Rect) -> egui::Rect {
    rect.shrink(PANE_GAP - 2.0)
}

/// Fractions are session UI state, independent of task state and saved game settings.
pub struct Splits {
    pub sidebar: f32,
    pub workspace: f32,
    pub settings: f32,
}

impl Default for Splits {
    fn default() -> Self {
        Self {
            sidebar: 0.10,
            workspace: 0.74,
            settings: 0.50,
        }
    }
}

pub fn split(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: &str,
    fraction: &mut f32,
    horizontal: bool,
    minimum: f32,
) -> [egui::Rect; 2] {
    let size = if horizontal {
        rect.width()
    } else {
        rect.height()
    };
    let limit = (minimum / size.max(1.0)).min(0.45);
    let origin = if horizontal { rect.left() } else { rect.top() };
    let mut position = origin + size * fraction.clamp(limit, 1.0 - limit);
    let divider = if horizontal {
        egui::Rect::from_min_max(
            egui::pos2(position - 3.0, rect.top()),
            egui::pos2(position + 3.0, rect.bottom()),
        )
    } else {
        egui::Rect::from_min_max(
            egui::pos2(rect.left(), position - 3.0),
            egui::pos2(rect.right(), position + 3.0),
        )
    };
    let response = ui.interact(divider, ui.id().with(id), egui::Sense::drag());
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(if horizontal {
            egui::CursorIcon::ResizeHorizontal
        } else {
            egui::CursorIcon::ResizeVertical
        });
    }
    if response.dragged() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let coordinate = if horizontal { pointer.x } else { pointer.y };
            *fraction = ((coordinate - origin) / size.max(1.0)).clamp(limit, 1.0 - limit);
            position = origin + size * *fraction;
        }
    }
    let color = if response.hovered() || response.dragged() {
        theme::palette(ui.visuals().dark_mode).accent
    } else {
        theme::palette(ui.visuals().dark_mode).border
    };
    let ends = if horizontal {
        [
            egui::pos2(position, rect.top()),
            egui::pos2(position, rect.bottom()),
        ]
    } else {
        [
            egui::pos2(rect.left(), position),
            egui::pos2(rect.right(), position),
        ]
    };
    ui.painter()
        .line_segment(ends, egui::Stroke::new(1.0, color));
    if horizontal {
        [
            egui::Rect::from_min_max(rect.min, egui::pos2(position, rect.bottom())),
            egui::Rect::from_min_max(egui::pos2(position, rect.top()), rect.max),
        ]
    } else {
        [
            egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), position)),
            egui::Rect::from_min_max(egui::pos2(rect.left(), position), rect.max),
        ]
    }
}

pub fn region(ui: &mut egui::Ui, rect: egui::Rect, id: &str, render: impl FnOnce(&mut egui::Ui)) {
    // Strokes straddle widget bounds. Reserve paint space on every pane edge,
    // while retaining the outer clip so overflow cannot leak into adjacent panes.
    let paint_inset = 2.0_f32.min(rect.width().min(rect.height()).max(0.0) / 2.0);
    render_region(ui, rect.shrink(paint_inset), rect, id, render);
}

/// For containers whose parent has already reserved the pane inset.
pub fn content_region(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: &str,
    render: impl FnOnce(&mut egui::Ui),
) {
    render_region(ui, rect, rect.expand(2.0), id, render);
}

fn render_region(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    clip: egui::Rect,
    id: &str,
    render: impl FnOnce(&mut egui::Ui),
) {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(id)
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(clip.intersect(ui.clip_rect()));
    render(&mut child);
}

pub fn workspace(
    ui: &mut egui::Ui,
    fraction: &mut f32,
    mut render: impl FnMut(&mut egui::Ui, Pane),
) {
    let rect = ui.available_rect_before_wrap();
    // Leave room for labels, counts and action controls when a divider is dragged.
    let [left, right] = split(ui, rect, "settings-status", fraction, true, 250.0);
    for (pane, rect, id) in [
        (
            Pane::Settings,
            egui::Rect::from_min_max(left.min, left.max - egui::vec2(PANE_GAP, 0.0)),
            "settings",
        ),
        (
            Pane::Status,
            egui::Rect::from_min_max(right.min + egui::vec2(PANE_GAP, 0.0), right.max),
            "status",
        ),
    ] {
        content_region(ui, rect, id, |ui| {
            let content_width = ui.available_width();
            if matches!(pane, Pane::Status) {
                egui::Frame::none()
                    .fill(theme::palette(ui.visuals().dark_mode).surface)
                    .stroke(egui::Stroke::new(1.0, theme::palette(ui.visuals().dark_mode).border))
                    .rounding(8.0)
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.set_width((content_width - 28.0).max(1.0));
                        ui.set_max_height((rect.height() - 28.0).max(1.0));
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.spacing_mut().interact_size.y = 22.0;
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                        render(ui, pane);
                    });
                return;
            }
            ui.spacing_mut().scroll = egui::style::ScrollStyle::floating();
            egui::ScrollArea::both()
                .id_salt(id)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_width(content_width);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                    ui.spacing_mut().item_spacing.y = 7.0;
                    render(ui, pane);
                });
        });
    }
}
