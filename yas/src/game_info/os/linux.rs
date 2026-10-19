use anyhow::{anyhow, Result};
use std::io::{stdin, IsTerminal};

use crate::game_info::{is_16x9, GameInfo, Platform, UI};
use crate::positioning::Rect;
use crate::utils::{self, X11WindowInfo};

fn is_window_cloud(title: &str) -> bool {
    title.starts_with("云")
}

/// Find the game window by exact title, mirroring the Windows flow in
/// `os/winodws.rs`: log every match, prefer unique matches, fall back to
/// interactive selection when ambiguous.
fn get_window(window_names: &[&str]) -> Result<(u32, bool)> {
    let windows = utils::enumerate_windows()?;

    let mut candidates: Vec<&X11WindowInfo> = windows
        .iter()
        .filter(|w| window_names.iter().any(|name| w.title == *name))
        .collect();

    let known: Vec<_> = candidates
        .iter()
        .copied()
        .filter(|w| w.class.contains("UnityWndClass") || w.class.contains("Qt5152QWindowIcon"))
        .collect();
    if !known.is_empty() {
        candidates = known;
    }

    if candidates.is_empty() {
        let wayland = std::env::var("WAYLAND_DISPLAY")
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        let extra = if wayland {
            " Native wine-wayland windows are invisible to X11: remove PROTON_ENABLE_WAYLAND=1 or Graphics=wayland and use XWayland."
        } else {
            ""
        };
        return Err(anyhow!(
            "Game window not found; make sure the game is running and not minimized.{extra}"
        ));
    }

    for w in &candidates {
        log_debug!(
            "匹配到窗口: title={:?}, class={:?}, id={:#x}",
            "Matched window: title={:?}, class={:?}, id={:#x}",
            w.title,
            w.class,
            w.window,
        );
    }

    if candidates.len() == 1 {
        let w = candidates[0];
        return Ok((w.window, is_window_cloud(&w.title)));
    }

    // Never wait on invisible stdin in a GUI worker.
    if !stdin().is_terminal() {
        anyhow::bail!("Multiple matching game windows; close extra clients and retry");
    }
    println!(
        "{}",
        crate::lang::localize(
            "找到多个符合名称的窗口，请手动选择窗口 / Multiple matching windows found, please select one:"
        )
    );
    for (i, w) in candidates.iter().enumerate() {
        println!("{}: {} [{}]", i, w.title, w.class);
    }
    let mut index = String::new();
    let idx = match stdin().read_line(&mut index) {
        Ok(_) => index.trim().parse::<usize>().unwrap_or(0),
        Err(_) => 0,
    };
    let idx = idx.min(candidates.len() - 1);
    let w = candidates[idx];
    Ok((w.window, is_window_cloud(&w.title)))
}

pub fn get_game_info(window_names: &[&str]) -> Result<GameInfo> {
    let (window_id, is_cloud) = get_window(window_names)?;

    let rect = utils::get_client_rect(window_id)?;

    if rect.width <= 0 || rect.height <= 0 || !is_16x9(rect.to_rect_usize().size()) {
        log_error!(
            "游戏窗口内部区域为 {}x{}，不是16:9比例。本工具仅支持16:9分辨率（如1920×1080、2560×1440、3840×2160）。\n\
             请在游戏设置中切换到16:9分辨率后重试。",
            "Game window client area is {}x{}, which is not 16:9. This tool only supports 16:9 aspect ratio \
             (e.g. 1920×1080, 2560×1440, 3840×2160).\n\
             Please switch to a 16:9 resolution in game settings and try again.",
            rect.width, rect.height,
        );
        return Err(anyhow!(
            "Unsupported client resolution {}x{}; use a 16:9 game window",
            rect.width,
            rect.height
        ));
    }

    Ok(GameInfo {
        window: Rect::new(rect.left, rect.top, rect.width, rect.height),
        is_cloud,
        ui: UI::Desktop,
        platform: Platform::Linux,
        window_id,
    })
}
