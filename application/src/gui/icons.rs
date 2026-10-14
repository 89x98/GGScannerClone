//! Phosphor Regular glyphs and the shared icon-button layout.
use super::theme;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

#[derive(Clone, Copy)]
pub enum Icon {
    Capture,
    Scan,
    Manager,
    Language,
    System,
    Sun,
    Moon,
    About,
    External,
    Minimize,
    Maximize,
    Restore,
    Close,
}

pub fn glyph(icon: Icon) -> &'static str {
    use egui_phosphor::regular as ph;
    match icon {
        Icon::Capture => ph::BROADCAST,
        Icon::Scan => ph::SCAN,
        Icon::Manager => ph::SLIDERS_HORIZONTAL,
        Icon::Language => ph::TRANSLATE,
        Icon::System => ph::DESKTOP,
        Icon::Sun => ph::SUN,
        Icon::Moon => ph::MOON,
        Icon::About => ph::INFO,
        Icon::External => ph::ARROW_UP_RIGHT,
        Icon::Minimize => ph::MINUS,
        Icon::Maximize => ph::SQUARE,
        Icon::Restore => ph::COPY_SIMPLE,
        Icon::Close => ph::X,
    }
}

pub fn add_to_fonts(fonts: &mut egui::FontDefinitions) {
    fonts.font_data.insert(
        "phosphor".into(),
        egui_phosphor::Variant::Regular.font_data().into(),
    );
    fonts.families.insert(
        egui::FontFamily::Name("icons".into()),
        vec!["phosphor".into()],
    );
}

pub fn paint(ui: &egui::Ui, rect: Rect, icon: Icon, color: Color32) {
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph(icon),
        egui::FontId::new(rect.width(), egui::FontFamily::Name("icons".into())),
        color,
    );
}

pub fn button(
    ui: &mut egui::Ui,
    label: &str,
    icon: Icon,
    size: Vec2,
    selected: bool,
    icon_only: bool,
) -> egui::Response {
    let p = theme::palette(ui.visuals().dark_mode);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    let hover = response.hovered();
    let pressed = response.is_pointer_button_down_on();
    let fill = if pressed {
        p.pressed
    } else if hover {
        p.hover
    } else if selected {
        p.selected
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 5., fill);
    if selected || hover || pressed {
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            5.,
            Stroke::new(1., if hover || pressed { p.accent } else { p.border }),
        );
    }
    let color = if selected || hover {
        p.accent
    } else {
        p.foreground
    };
    let external = matches!(icon, Icon::External);
    let icon_center = if icon_only {
        rect.center()
    } else if external {
        Pos2::new(rect.right() - 14., rect.center().y)
    } else {
        Pos2::new(rect.left() + 19., rect.center().y)
    };
    paint(
        ui,
        Rect::from_center_size(icon_center, Vec2::splat(18.)),
        icon,
        color,
    );
    if !icon_only {
        ui.painter().text(
            Pos2::new(
                rect.left() + if external { 8. } else { 36. },
                rect.center().y,
            ),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(14.),
            color,
        );
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}
