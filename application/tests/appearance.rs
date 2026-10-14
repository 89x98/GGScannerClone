use eframe::egui;
use good_tools_app::{
    config::{Appearance, ApplicationConfigStore, APPLICATION_CONFIG_SCHEMA_VERSION},
    gui::theme,
};

#[test]
fn old_configs_gain_system_theme_without_changing_saved_task_settings() {
    let root = std::env::temp_dir().join(format!("ggscanner-appearance-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("app.json");
    for version in 1..=5 {
        let fixture = serde_json::json!({
            "schemaVersion":version,
            "navigation":{"activeGame":"starRail","genshinTab":"manager","starRailTab":"scanner"},
            "starRail":{"trailblazerName":"Player","trailblazerGender":"Stelle","outputDir":"D:/exports","hdrMode":true,"scanCharacters":false,"captureOnlyKeepLatestExport":false,"scanSaveOnCancel":true,"managerPort":9123}
        });
        std::fs::write(&path, fixture.to_string()).unwrap();
        let mut store = ApplicationConfigStore::load(&path).unwrap();
        assert_eq!(
            store.config.schema_version,
            APPLICATION_CONFIG_SCHEMA_VERSION
        );
        assert_eq!(store.config.appearance, Appearance::System);
        assert!(!store.config.star_rail.scan_characters);
        assert!(!store.config.star_rail.capture_only_keep_latest_export);
        assert!(store.config.star_rail.hdr_mode);
        assert!(store.config.star_rail.scan_save_on_cancel);
        assert_eq!(store.config.star_rail.manager_port, 9123);
        assert_eq!(store.config.star_rail.trailblazer_name, "Player");
        assert_eq!(store.config.star_rail.output_dir, "D:/exports");
        for appearance in [Appearance::Light, Appearance::Dark, Appearance::System] {
            store.config.appearance = appearance;
            store.persist_now().unwrap();
            assert_eq!(
                ApplicationConfigStore::load(&path).unwrap().config,
                store.config
            );
        }
    }
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn system_theme_tracks_os_changes_and_manual_choices_override_them() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    theme::install(&ctx, Appearance::System);
    for (appearance, os, dark) in [
        (Appearance::System, egui::Theme::Light, false),
        (Appearance::System, egui::Theme::Dark, true),
        (Appearance::Light, egui::Theme::Dark, false),
        (Appearance::Dark, egui::Theme::Light, true),
        (Appearance::System, egui::Theme::Light, false),
    ] {
        ctx.set_theme(theme::preference(appearance));
        let _ = ctx.run(
            egui::RawInput {
                system_theme: Some(os),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    assert_eq!(ui.visuals().dark_mode, dark);
                    let p = theme::palette(dark);
                    assert_eq!(ui.visuals().panel_fill, p.background);
                    assert_eq!(ui.visuals().window_fill, p.surface);
                    assert_eq!(ui.visuals().text_color(), p.foreground);
                    theme::primary_action(ui, true, "Start");
                });
            },
        );
    }
}

#[test]
fn sidebar_theme_menu_selects_each_mode_with_real_pointer_events() {
    use good_tools_app::{
        config::GameNavigation,
        gui::{shell, state::Lang},
    };
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    theme::install(&ctx, Appearance::System);
    let mut appearance = Appearance::System;
    let mut navigation = GameNavigation::default();
    let mut lang = Lang::En;
    let selected = std::cell::Cell::new(Appearance::System);
    let mut draw = |events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(760., 500.),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.set_width(150.0);
                    shell::sidebar(ui, &mut lang, &mut navigation, &mut appearance)
                });
                selected.set(appearance);
            },
        )
    };
    draw(vec![]);
    let frame = draw(vec![]);
    let button = frame
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == egui_phosphor::regular::DESKTOP => {
                Some(t.pos + t.galley.size() / 2.0)
            },
            _ => None,
        })
        .unwrap();
    let click = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    for label in ["Light theme", "Dark theme", "System theme"] {
        draw(vec![egui::Event::PointerMoved(button), click(button, true)]);
        draw(vec![click(button, false)]);
        draw(vec![]);
        draw(vec![]);
        let frame = draw(vec![]);
        fn collect(shape: &egui::Shape, texts: &mut Vec<(String, egui::Pos2)>) {
            match shape {
                egui::Shape::Text(t) => {
                    texts.push((t.galley.job.text.clone(), t.pos + t.galley.size() / 2.))
                },
                egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, texts)),
                _ => {},
            }
        }
        let mut texts = Vec::new();
        frame
            .shapes
            .iter()
            .for_each(|s| collect(&s.shape, &mut texts));
        let option = texts
            .iter()
            .find(|(text, _)| text == label)
            .unwrap_or_else(|| {
                panic!(
                    "missing {label}; open={}; captions={texts:?}",
                    ctx.memory(|m| m.is_popup_open(egui::Id::new("appearance-menu")))
                )
            })
            .1;
        draw(vec![egui::Event::PointerMoved(option), click(option, true)]);
        draw(vec![click(option, false)]);
        draw(vec![]);
        assert_eq!(
            selected.get(),
            match label {
                "Light theme" => Appearance::Light,
                "Dark theme" => Appearance::Dark,
                _ => Appearance::System,
            }
        );
        assert!(!ctx.memory(|m| m.is_popup_open(egui::Id::new("appearance-menu"))));
    }
    drop(draw);
    assert_eq!(appearance, Appearance::System);
}
