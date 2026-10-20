mod common;
use common::HeadlessContext;
use eframe::egui;
use good_tools_app::{
    config::{Appearance, ApplicationConfigStore, APPLICATION_CONFIG_SCHEMA_VERSION},
    gui::theme,
};

#[test]
fn per_game_tabs_migrate_to_the_last_active_games_page() {
    let root = std::env::temp_dir().join(format!("ggscanner-shared-tab-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("app.json");
    for version in 1..=6 {
        for (game, expected) in [
            ("genshin", good_tools_app::config::ToolTab::Scanner),
            ("starRail", good_tools_app::config::ToolTab::Manager),
        ] {
            let fixture = serde_json::json!({
                "schemaVersion": version,
                "appearance": "light",
                "navigation": {"activeGame": game, "genshinTab": "scanner", "starRailTab": "manager"},
                "starRail": {"outputDir": "D:/exports", "trailblazerName": "Player"}
            });
            std::fs::write(&path, fixture.to_string()).unwrap();
            let mut store = ApplicationConfigStore::load(&path).unwrap();
            assert_eq!(store.config.navigation.selected_tab, expected);
            store.config.navigation.active_game = good_tools_app::config::Game::Genshin;
            assert_eq!(store.config.navigation.active_tab(), expected);
            store.config.navigation.active_game = good_tools_app::config::Game::StarRail;
            assert_eq!(store.config.navigation.active_tab(), expected);
            assert_eq!(store.config.appearance, Appearance::Light);
            assert_eq!(store.config.star_rail.output_dir, "D:/exports");
            store.persist_now().unwrap();
            let saved: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            assert_eq!(saved["schemaVersion"], APPLICATION_CONFIG_SCHEMA_VERSION);
            assert!(saved["navigation"].get("genshinTab").is_none());
            assert!(saved["navigation"].get("starRailTab").is_none());
            assert_eq!(
                ApplicationConfigStore::load(&path)
                    .unwrap()
                    .config
                    .navigation
                    .selected_tab,
                expected
            );
        }
    }
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn icon_family_has_a_replacement_glyph_without_losing_icons() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    ctx.run_headless(Default::default(), |ui| {
        let font = egui::FontId::new(18.0, egui::FontFamily::Name("icons".into()));
        ui.fonts_mut(|fonts| {
            assert!(fonts.has_glyph(&font, '?'));
            assert!(fonts.has_glyphs(
                &font,
                good_tools_app::gui::icons::glyph(good_tools_app::gui::icons::Icon::Scan,)
            ));
        });
    });
}

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
        let _ = ctx.run_headless(
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
fn sidebar_theme_toggle_switches_on_one_click_in_its_own_row() {
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
        ctx.run_headless(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(760., 500.),
                )),
                events,
                system_theme: Some(egui::Theme::Light),
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
    let click = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    for (label, expected) in [("Dark", Appearance::Dark), ("Light", Appearance::Light)] {
        draw(vec![]);
        let frame = draw(vec![]);
        let texts: Vec<_> = frame
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) => {
                    Some((t.galley.job.text.as_str(), t.pos + t.galley.size() / 2.))
                },
                _ => None,
            })
            .collect();
        let button = texts.iter().find(|(text, _)| *text == label).unwrap().1;
        let language = texts.iter().find(|(text, _)| *text == "中文").unwrap().1;
        assert!(button.y > language.y + 20.0);
        assert!(!texts.iter().any(|(text, _)| *text == "System theme"));
        draw(vec![egui::Event::PointerMoved(button), click(button, true)]);
        draw(vec![click(button, false)]);
        assert_eq!(selected.get(), expected);
    }
    drop(draw);
    assert_eq!(appearance, Appearance::Light);
}

#[cfg(target_os = "linux")]
#[test]
fn explicit_theme_choice_is_not_overridden_by_the_desktop_theme() {
    // Linux has no winit-reported system theme, so the portal watcher supplies
    // one. That must refine `System` only — a user's explicit Light/Dark pick
    // has to survive a desktop theme that disagrees.
    for (appearance, desktop, expected) in [
        (
            Appearance::System,
            Some(egui::Theme::Dark),
            egui::ThemePreference::Dark,
        ),
        (
            Appearance::System,
            Some(egui::Theme::Light),
            egui::ThemePreference::Light,
        ),
        (Appearance::System, None, egui::ThemePreference::System),
        (
            Appearance::Light,
            Some(egui::Theme::Dark),
            egui::ThemePreference::Light,
        ),
        (
            Appearance::Dark,
            Some(egui::Theme::Light),
            egui::ThemePreference::Dark,
        ),
        (Appearance::Dark, None, egui::ThemePreference::Dark),
    ] {
        assert_eq!(
            good_tools_app::gui::linux_theme::effective_preference(appearance, desktop),
            expected,
            "appearance={appearance:?} desktop={desktop:?}"
        );
    }

    // And the resolved preference actually drives the applied style.
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    ctx.set_theme(good_tools_app::gui::linux_theme::effective_preference(
        Appearance::Light,
        Some(egui::Theme::Dark),
    ));
    let _ = ctx.run_headless(
        egui::RawInput {
            system_theme: Some(egui::Theme::Dark),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                assert!(!ui.visuals().dark_mode);
                assert_eq!(ui.visuals().panel_fill, theme::palette(false).background);
            });
        },
    );
}
