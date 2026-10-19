use eframe::egui;

use super::{
    state::Lang,
    task_progress::{self, Step, StepState},
};

/// Both capture adapters report the same user journey from their actual monitor state.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Stage {
    Ready,
    Starting,
    Login,
    Data,
    Achievements,
    Stopping,
    Exporting,
    Done,
    Stopped,
    Failed,
}

impl Stage {
    pub fn headline(self, lang: Lang) -> &'static str {
        match self {
            Self::Ready => lang.t("请先关闭游戏", "Close the game first"),
            Self::Starting => lang.t("正在准备抓包", "Preparing capture"),
            Self::Login => lang.t("请启动游戏并登录", "Launch the game and log in"),
            Self::Data => lang.t("正在接收游戏数据", "Receiving game data"),
            Self::Achievements => lang.t("请在游戏中打开成就", "Open Achievements in the game"),
            Self::Stopping => lang.t("正在停止抓包", "Stopping capture"),
            Self::Exporting => lang.t("正在导出数据", "Exporting data"),
            Self::Done => lang.t("数据已导出", "Data exported"),
            Self::Stopped => lang.t("抓包已停止 · 未导出", "Capture stopped · no export"),
            Self::Failed => lang.t("抓包未完成", "Capture failed"),
        }
    }
}

pub(super) fn waiting(
    key_exchange_captured: bool,
    missing_inventory: bool,
    missing_achievements: bool,
) -> Stage {
    if !key_exchange_captured {
        Stage::Login
    } else if !missing_inventory && missing_achievements {
        Stage::Achievements
    } else {
        Stage::Data
    }
}

pub(super) fn show_connection(
    ui: &mut egui::Ui,
    lang: Lang,
    stage: Stage,
    key_exchange_captured: bool,
) {
    let mut step = Step::new("key_exchange", "登录握手", "Login handshake");
    step.show_count = false;
    step.state = if key_exchange_captured {
        StepState::Complete
    } else if matches!(stage, Stage::Stopped | Stage::Failed) {
        StepState::Interrupted
    } else if stage == Stage::Login {
        StepState::Running
    } else {
        StepState::Pending
    };
    task_progress::row(ui, lang, &step);
}

pub(super) struct Diagnostics<'a> {
    pub active: bool,
    pub ready: bool,
    pub elapsed: u64,
    pub packets: usize,
    pub commands: usize,
    pub error: Option<&'a str>,
    pub details: String,
}

pub(super) fn show_diagnostics(ui: &mut egui::Ui, lang: Lang, data: Diagnostics<'_>) {
    let hint = if !data.active {
        lang.t(
            "本次抓包已结束，以下为停止前的诊断信息。",
            "Capture ended; diagnostics below describe the last attempt.",
        )
    } else if !data.ready {
        lang.t(
            "Windows 监听器尚未就绪",
            "The Windows listener is not ready yet",
        )
    } else if data.packets == 0 {
        lang.t(
            "监听已启动，尚未收到游戏流量。请在本机启动游戏并登录。",
            "Listener ready; no game traffic yet. Launch the game on this PC and log in.",
        )
    } else if data.commands == 0 {
        lang.t("已收到流量，尚未成功解码游戏消息。若持续如此，请重新登录并更新抓包器。", "Traffic received; no game messages decoded yet. If this persists, capture a new login and update the scanner.")
    } else {
        lang.t(
            "已成功解码游戏消息，正在识别所选数据。",
            "Game messages decoded; identifying selected data.",
        )
    };
    ui.label(hint);
    ui.label(lang.t(
        &format!(
            "已等待 {} 秒 · 收到 {} 个包 · 解码 {} 条消息",
            data.elapsed, data.packets, data.commands
        ),
        &format!(
            "{}s elapsed · {} packets · {} decoded messages",
            data.elapsed, data.packets, data.commands
        ),
    ));
    ui.collapsing(lang.t("抓包诊断", "Capture diagnostics"), |ui| {
        if let Some(error) = data.error {
            ui.label(lang.t(
                "最近一次底层错误（不代表所有包均失败）：",
                "Last transport error (other packets may have succeeded):",
            ));
            ui.label(error);
        }
        if ui
            .button(lang.t("复制诊断信息", "Copy diagnostics"))
            .clicked()
        {
            ui.ctx().copy_text(data.details.clone());
        }
        ui.label(lang.t(
            "日志保存在 log 文件夹的 run_*.log 中。",
            "Logs are saved as log/run_*.log.",
        ));
    });
}

#[cfg(test)]
pub(crate) mod feedback_tests {
    use super::*;

    #[test]
    fn diagnostics_show_actual_traffic_and_decoding_state() {
        for (ready, packets, commands, expected) in [
            (false, 0, 0, "listener is not ready yet"),
            (true, 0, 0, "no game traffic yet"),
            (true, 10, 0, "no game messages decoded yet"),
            (true, 10, 3, "Game messages decoded"),
        ] {
            let content = texts(|ui| {
                show_diagnostics(
                    ui,
                    Lang::En,
                    Diagnostics {
                        active: true,
                        ready,
                        elapsed: 15,
                        packets,
                        commands,
                        error: None,
                        details: String::new(),
                    },
                )
            });
            assert!(
                content.iter().any(|text| text.contains(expected)),
                "{content:?}"
            );
        }
    }

    #[test]
    fn handshake_step_never_shows_a_meaningless_item_count() {
        let content = texts(|ui| show_connection(ui, Lang::En, Stage::Login, false));
        assert!(content.iter().any(|text| text == "Login handshake"));
        assert!(!content.iter().any(|text| text == "0"));
    }

    #[test]
    fn only_a_captured_exchange_marks_the_handshake_complete() {
        for stage in [Stage::Login, Stage::Data, Stage::Done, Stage::Failed] {
            let missing = colored_texts(|ui| show_connection(ui, Lang::En, stage, false));
            assert!(missing.iter().any(|(text, color)| text == "Login handshake"
                && *color != super::super::theme::palette(true).success));
            let captured = colored_texts(|ui| show_connection(ui, Lang::En, stage, true));
            assert!(captured
                .iter()
                .any(|(text, color)| text == "Login handshake"
                    && *color == super::super::theme::palette(true).success));
        }
    }

    pub(crate) fn texts(draw: impl FnMut(&mut egui::Ui)) -> Vec<String> {
        colored_texts(draw)
            .into_iter()
            .map(|(text, _)| text)
            .collect()
    }

    pub(crate) fn colored_texts(
        mut draw: impl FnMut(&mut egui::Ui),
    ) -> Vec<(String, egui::Color32)> {
        fn collect(shape: &egui::Shape, texts: &mut Vec<(String, egui::Color32)>) {
            match shape {
                egui::Shape::Text(text) => {
                    for section in &text.galley.job.sections {
                        texts.push((
                            text.galley.job.text
                                [section.byte_range.start.0..section.byte_range.end.0]
                                .into(),
                            section.format.color,
                        ));
                    }
                },
                egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, texts)),
                _ => {},
            }
        }
        let ctx = egui::Context::default();
        super::super::theme::setup(&ctx);
        let mut texts = Vec::new();
        for _ in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(370.0, 700.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| draw(ui));
                },
            );
            output.textures_delta.clear();
            texts.clear();
            for clipped in output.shapes {
                collect(&clipped.shape, &mut texts);
            }
        }
        texts
    }
}
