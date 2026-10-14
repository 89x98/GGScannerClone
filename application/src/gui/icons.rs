//! Small resolution-independent icons, drawn with egui's existing renderer.
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
}

pub fn paint(ui: &egui::Ui, rect: Rect, icon: Icon, color: Color32) {
    let painter = ui.painter();
    let stroke = Stroke::new(1.4, color);
    let point = |x: f32, y: f32| rect.min + Vec2::new(x, y) * (rect.width() / 20.0);
    let line = |a: (f32, f32), b: (f32, f32)| {
        painter.line_segment([point(a.0, a.1), point(b.0, b.1)], stroke)
    };
    match icon {
        Icon::Scan => {
            for (x, y, dx, dy) in [
                (3., 3., 1., 1.),
                (17., 3., -1., 1.),
                (3., 17., 1., -1.),
                (17., 17., -1., -1.),
            ] {
                line((x, y), (x + 4. * dx, y));
                line((x, y), (x, y + 4. * dy));
            }
            line((6., 10.), (14., 10.));
        },
        Icon::Capture => {
            painter.circle_stroke(rect.center(), rect.width() * 0.36, stroke);
            painter.circle_stroke(rect.center(), rect.width() * 0.18, stroke);
            line((10., 10.), (16., 4.));
            painter.circle_filled(point(7., 13.), rect.width() * 0.07, color);
        },
        Icon::Manager => {
            for (y, x) in [(5., 7.), (10., 13.), (15., 9.)] {
                line((3., y), (x - 2., y));
                line((x + 2., y), (17., y));
                painter.circle_stroke(point(x, y), rect.width() * 0.1, stroke);
            }
        },
        Icon::Language => {
            painter.text(
                point(1., 7.),
                egui::Align2::LEFT_CENTER,
                "文",
                egui::FontId::proportional(rect.width() * 0.65),
                color,
            );
            painter.text(
                point(19., 14.),
                egui::Align2::RIGHT_CENTER,
                "A",
                egui::FontId::proportional(rect.width() * 0.65),
                color,
            );
        },
        Icon::System => {
            painter.rect_stroke(
                Rect::from_min_max(point(2., 3.), point(18., 14.)),
                2.,
                stroke,
            );
            line((10., 14.), (10., 18.));
            line((6., 18.), (14., 18.));
        },
        Icon::Sun => {
            painter.circle_stroke(rect.center(), rect.width() * 0.21, stroke);
            for n in 0..8 {
                let angle = n as f32 * std::f32::consts::TAU / 8.;
                let dir = Vec2::angled(angle);
                painter.line_segment(
                    [
                        rect.center() + dir * rect.width() * 0.34,
                        rect.center() + dir * rect.width() * 0.46,
                    ],
                    stroke,
                );
            }
        },
        Icon::Moon => {
            let points = [
                (13., 2.),
                (8., 3.),
                (4., 7.),
                (3., 11.),
                (5., 15.),
                (9., 17.),
                (14., 16.),
                (17., 12.),
                (12., 13.),
                (8., 10.),
                (8., 6.),
                (13., 2.),
            ];
            painter.add(egui::Shape::line(
                points.into_iter().map(|(x, y)| point(x, y)).collect(),
                stroke,
            ));
        },
        Icon::About => {
            painter.circle_stroke(rect.center(), rect.width() * 0.38, stroke);
            painter.circle_filled(point(10., 6.), rect.width() * 0.06, color);
            line((10., 9.), (10., 14.));
        },
        Icon::External => {
            line((5., 15.), (15., 5.));
            line((8., 5.), (15., 5.));
            line((15., 5.), (15., 12.));
        },
    }
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
