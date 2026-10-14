//! Phosphor Regular glyphs and the shared icon-button layout.
use super::theme;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

egui_phosphor::subset! {
    pub mod bundled {
        use regular::{BROADCAST, SCAN, SLIDERS_HORIZONTAL, TRANSLATE, SUN, MOON, INFO, QUESTION, ARROW_SQUARE_OUT, MINUS, SQUARE, COPY_SIMPLE, X};
    }
}

#[derive(Clone, Copy)]
pub enum Icon {
    Capture,
    Scan,
    Manager,
    Language,
    Sun,
    Moon,
    About,
    Help,
    External,
    Minimize,
    Maximize,
    Restore,
    Close,
}

pub fn glyph(icon: Icon) -> &'static str {
    use bundled::regular as ph;
    match icon {
        Icon::Capture => ph::BROADCAST,
        Icon::Scan => ph::SCAN,
        Icon::Manager => ph::SLIDERS_HORIZONTAL,
        Icon::Language => ph::TRANSLATE,
        Icon::Sun => ph::SUN,
        Icon::Moon => ph::MOON,
        Icon::About => ph::INFO,
        Icon::Help => ph::QUESTION,
        Icon::External => ph::ARROW_SQUARE_OUT,
        Icon::Minimize => ph::MINUS,
        Icon::Maximize => ph::SQUARE,
        Icon::Restore => ph::COPY_SIMPLE,
        Icon::Close => ph::X,
    }
}

pub fn add_to_fonts(fonts: &mut egui::FontDefinitions) {
    fonts.font_data.insert(
        "phosphor".into(),
        egui::FontData::from_static(&bundled::regular::FONT).into(),
    );
    // egui resolves a replacement character for every family on startup.
    // The icon subset has none, so reuse the bundled text fonts as fallback.
    let mut icon_fonts = vec!["phosphor".into()];
    icon_fonts.extend(
        fonts.families[&egui::FontFamily::Proportional]
            .iter()
            .cloned(),
    );
    fonts
        .families
        .insert(egui::FontFamily::Name("icons".into()), icon_fonts);
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
            egui::StrokeKind::Inside,
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
        let text_width = ui.fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(label.into(), egui::FontId::proportional(14.), color)
                .size()
                .x
        });
        Pos2::new(rect.left() + 16. + text_width + 2. + 9., rect.center().y)
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
                rect.left() + if external { 16. } else { 36. },
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
