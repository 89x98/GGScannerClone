use eframe::egui;
use hsr_scanner::TrailblazerGender;

use crate::config::StarRailSettings;

use super::{star_rail_state::StarRailState, star_rail_worker, state::Lang, widgets, worker};

pub fn show_status(
    ui: &mut egui::Ui,
    lang: Lang,
    settings: &mut StarRailSettings,
    state: &mut StarRailState,
    game_busy: bool,
    restart_required: bool,
) {
    let is_running = state.scan_running();
    if restart_required {
        super::theme::restart_required(
            ui,
            lang,
            state
                .scan_handle
                .as_ref()
                .and_then(super::worker::TaskHandle::native_failure)
                .as_ref(),
        );
        return;
    }
    let mut status = worker::try_task_status(&state.scan_status);
    if matches!(status, Some(super::state::TaskStatus::Idle)) {
        let reason = if !settings.scan_characters
            && !settings.scan_light_cones
            && !settings.scan_relics_and_ornaments
        {
            Some(lang.t("请选择扫描内容", "Select scan targets"))
        } else if settings.missing_trailblazer() {
            Some(lang.t(
                "请填写游戏内昵称并选择性别",
                "Enter your in-game nickname and choose a gender",
            ))
        } else {
            None
        };
        if let Some(reason) = reason {
            status = Some(super::state::TaskStatus::AwaitingInput(
                super::state::UiText::new(reason, reason),
            ));
        }
    }
    widgets::status_content(ui, "hsr-scan-status", 2, |ui| {
        super::theme::task_status(
            ui,
            lang,
            status.as_ref(),
            lang.t("准备扫描", "Ready to scan"),
        );
        if let Ok(progress) = state.scan_progress.try_lock() {
            let mut display = progress.clone();
            if matches!(status, Some(super::state::TaskStatus::Failed(_))) {
                display.interrupt_unfinished();
            }
            super::task_progress::show(ui, lang, &display);
        }
    });
    ui.add_space(12.0);
    ui.add_enabled_ui(!is_running && !game_busy, |ui| {
        widgets::scan_export_options(
            ui,
            lang,
            &mut settings.scan_only_keep_latest_export,
            &mut settings.scan_save_on_cancel,
        );
    });
    ui.add_space(4.0);
    ui.scope(|ui| {
        if is_running {
            let stopping = state
                .scan_handle
                .as_ref()
                .is_some_and(super::worker::TaskHandle::is_stopping);
            if super::theme::primary_action(
                ui,
                !stopping,
                if stopping {
                    lang.t("正在停止", "Stopping")
                } else {
                    lang.t("停止扫描", "Stop scan")
                },
            )
            .clicked()
            {
                if let Some(handle) = &state.scan_handle {
                    handle.stop();
                }
            }
        } else {
            let has_target = settings.scan_characters
                || settings.scan_light_cones
                || settings.scan_relics_and_ornaments;
            if super::theme::primary_action(
                ui,
                !game_busy && has_target && !settings.missing_trailblazer(),
                if matches!(status, Some(super::state::TaskStatus::Failed(_))) {
                    lang.t("重试", "Retry")
                } else {
                    lang.t("开始扫描", "Start scan")
                },
            )
            .clicked()
            {
                state.scan_handle = Some(star_rail_worker::spawn_scan(
                    settings,
                    state.scan_status.clone(),
                    state.scan_progress.clone(),
                ));
            }
        }
    });
}

pub fn show_settings(
    ui: &mut egui::Ui,
    lang: Lang,
    settings: &mut StarRailSettings,
    is_running: bool,
) {
    let game_busy = false;

    widgets::form(ui, |ui| {
        ui.add_enabled_ui(!is_running && !game_busy, |ui| {
            widgets::group(ui, lang.t("扫描内容", "Scan targets"), |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut settings.scan_characters, lang.t("角色", "Characters"));
                    ui.checkbox(
                        &mut settings.scan_light_cones,
                        lang.t("光锥", "Light Cones"),
                    );
                    ui.checkbox(
                        &mut settings.scan_relics_and_ornaments,
                        lang.t("遗器与饰品", "Relics"),
                    )
                    .on_hover_text(
                        lang.t("隧洞遗器与位面饰品", "Cavern Relics and Planar Ornaments"),
                    );
                });
            });
            if settings.scan_characters {
                ui.separator();
                widgets::group(ui, lang.t("角色信息", "Character info"), |ui| {
                    trailblazer_row(ui, lang, settings, true);
                });
            }
            ui.separator();
            if ui
                .checkbox(
                    &mut settings.hdr_mode,
                    lang.t("游戏使用 HDR", "Game uses HDR"),
                )
                .changed()
            {
                settings.set_hdr_mode(settings.hdr_mode);
            }
        });
    });

    timing_settings(ui, lang, settings, is_running);
    widgets::fold(ui, lang.t("高级选项", "Advanced"), |ui| {
        ui.add_enabled_ui(!is_running && !game_busy, |ui| {
            widgets::scan_debug_options(
                ui,
                lang,
                &mut settings.verbose,
                &mut settings.stop_on_failure,
                &mut settings.dump_images,
            );
            ui.separator();
            widgets::scan_limits(
                ui,
                lang,
                &mut [
                    (lang.t("角色", "Characters"), &mut settings.max_characters),
                    (lang.t("光锥", "Light Cones"), &mut settings.max_light_cones),
                    (lang.t("遗器与饰品", "Relics"), &mut settings.max_gear),
                ],
            );
        });
    });
}

/// The Trailblazer can be renamed in-game, like Genshin's Traveler, and the
/// character screen does not show which gender this account plays.
pub(super) fn trailblazer_row(
    ui: &mut egui::Ui,
    lang: Lang,
    settings: &mut StarRailSettings,
    required: bool,
) {
    widgets::field_row(
        ui,
        if required {
            lang.t("游戏内昵称*", "In-game nickname*")
        } else {
            lang.t("游戏内昵称", "In-game nickname")
        },
        |ui| {
            if required && settings.trailblazer_name.trim().is_empty() {
                ui.visuals_mut().widgets.inactive.bg_stroke.color = super::theme::WARNING;
            }
            ui.add(
                widgets::singleline_input(&mut settings.trailblazer_name)
                    .desired_width(ui.available_width())
                    .min_size(egui::vec2(0.0, 26.0)),
            );
        },
    );
    widgets::field_row(
        ui,
        if required {
            lang.t("性别*", "Gender*")
        } else {
            lang.t("性别", "Gender")
        },
        |ui| {
            if required && settings.trailblazer_gender.is_none() {
                ui.visuals_mut().widgets.inactive.bg_stroke.color = super::theme::WARNING;
            }
            egui::ComboBox::from_id_salt("star_rail_trailblazer_gender")
                .width(ui.available_width() - ui.spacing().button_padding.x * 2.0)
                .selected_text(match settings.trailblazer_gender {
                    Some(gender) => trailblazer_gender_label(lang, gender),
                    None => lang.t("请选择", "Choose"),
                })
                .show_ui(ui, |ui| {
                    for gender in [TrailblazerGender::Stelle, TrailblazerGender::Caelus] {
                        ui.selectable_value(
                            &mut settings.trailblazer_gender,
                            Some(gender),
                            trailblazer_gender_label(lang, gender),
                        );
                    }
                });
        },
    );
}

fn trailblazer_gender_label(lang: Lang, gender: TrailblazerGender) -> &'static str {
    match gender {
        TrailblazerGender::Stelle => lang.t("星（女）", "Stelle (female)"),
        TrailblazerGender::Caelus => lang.t("穹（男）", "Caelus (male)"),
    }
}

pub(super) fn timing_settings(
    ui: &mut egui::Ui,
    lang: Lang,
    settings: &mut StarRailSettings,
    is_running: bool,
) {
    let game_busy = false;
    widgets::fold(ui, lang.t("延迟设置", "Timing"), |ui| {
        ui.add_enabled_ui(!is_running && !game_busy, |ui| {
                        let defaults = hsr_scanner::scan_timing::ScanTimings::default();
                        let timings = &mut settings.timings;
                        {
                            widgets::delay_group(ui, "hsr_timing_0", lang.t("界面操作", "Navigation"), lang, &mut [
                                (lang.t("打开界面", "Open screen"), &mut timings.menu_open_ms, defaults.menu_open_ms, lang.t("打开背包或角色界面后的总等待时间", "Total wait after opening Inventory or Characters")),
                                (lang.t("关闭界面", "Close screen"), &mut timings.menu_close_ms, defaults.menu_close_ms, lang.t("关闭界面或弹窗后的等待时间", "Wait after closing a menu or dialog")),
                                (lang.t("输入就绪", "Input ready"), &mut timings.input_settle_ms, defaults.input_settle_ms, lang.t("切换鼠标操作模式或激活窗口后等待输入生效", "Wait after activating the window or switching input mode")),
                                (lang.t("背包分类切换", "Inventory tab"), &mut timings.inventory_tab_ms, defaults.inventory_tab_ms, lang.t("切换光锥或遗器分类后的等待时间", "Wait after switching Light Cone or Relic tabs")),
                                (lang.t("详情面板切换", "Panel switch"), &mut timings.panel_switch_ms, defaults.panel_switch_ms, lang.t("角色详情、星魂以及首次选中背包物品共用此等待", "Shared wait for Character details, Eidolons and the first inventory selection")),
                                (lang.t("打开行迹", "Open Traces"), &mut timings.traces_open_ms, defaults.traces_open_ms, lang.t("行迹界面加载后的等待时间", "Wait for the Traces screen to load")),
                                (lang.t("角色栏翻页", "Character page"), &mut timings.character_page_ms, defaults.character_page_ms, lang.t("角色栏拖动后的等待，扫描和拖动诊断共用", "Wait after dragging the Character bar, shared with the drag diagnostic")),
                                (lang.t("切换角色", "Character switch"), &mut timings.character_switch_ms, defaults.character_switch_ms, lang.t("点击下一名角色后，开始检查名字变化前的等待", "Wait after clicking the next Character before checking the name change")),
                            ]);
                            widgets::delay_group(ui, "hsr_timing_1", lang.t("截图与验证", "Capture & Verification"), lang, &mut [
                                (lang.t("截图间隔", "Capture interval"), &mut timings.capture_interval_ms, defaults.capture_interval_ms, lang.t("比较稳定画面或选中框的两次截图之间的等待", "Wait between screenshots used to confirm a stable panel or selection")),
                                (lang.t("按键后等待", "Key settle"), &mut timings.key_settle_ms, defaults.key_settle_ms, lang.t("发送下一项按键后，开始检查画面前的等待", "Wait after next-item input before checking the screen")),
                                (lang.t("面板检查间隔", "Panel poll interval"), &mut timings.poll_interval_ms, defaults.poll_interval_ms, lang.t("等待物品或角色切换时检查画面的间隔", "Interval for checking an item or Character transition")),
                                (lang.t("选中框动画", "Selection animation"), &mut timings.selection_settle_ms, defaults.selection_settle_ms, lang.t("相同物品副本之间切换时，选中框动画的等待时间", "Selection animation wait when moving between identical item copies")),
                                (lang.t("面板等待上限", "Panel timeout"), &mut timings.panel_timeout_ms, defaults.panel_timeout_ms, lang.t("单次切换等待画面更新的最长时间", "Maximum wait for one item or Character transition")),
                                (lang.t("界面检查间隔", "Menu poll interval"), &mut timings.menu_poll_interval_ms, defaults.menu_poll_interval_ms, lang.t("界面标题尚未加载时重新检查的间隔", "Interval for rechecking a menu title while it loads")),
                                (lang.t("标记切换", "Status toggle"), &mut timings.status_toggle_ms, defaults.status_toggle_ms, lang.t("管理器切换锁定或弃置标记后共用的等待", "Shared manager wait after toggling Lock or Discard")),
                            ]);
                        }
                    });
    });
}
