//! Replay dumped HSR commands (or raw frames with --raw) through the production decoder
//! and optionally retain decrypted messages in a separate diagnostic directory.
use std::{env, fs, path::PathBuf};

use hsr_scanner::{
    load_embedded_gilore_reference,
    packet_capture::{CaptureTargets, HsrPacketDecoder},
};

fn main() {
    let root = PathBuf::from(
        env::args()
            .nth(1)
            .expect("usage: analyze_dump <session dir> [--raw]"),
    );
    let raw = env::args().skip(2).any(|arg| arg == "--raw");
    let options: Vec<String> = env::args().skip(2).collect();
    let option_path = |name: &str| {
        options
            .windows(2)
            .find(|pair| pair[0] == name)
            .map(|pair| PathBuf::from(&pair[1]))
    };
    let output = option_path("--output-dir").unwrap_or_else(|| root.clone());
    let prefix = if raw { "raw_" } else { "hsr_" };
    let mut files: Vec<_> = fs::read_dir(&root)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".bin"))
        })
        .collect();
    files.sort();
    eprintln!(
        "replaying {} {} from {}",
        files.len(),
        if raw {
            "raw frames"
        } else {
            "decrypted commands"
        },
        root.display()
    );

    let references = if raw {
        let references = hsr_scanner::data_cache::load_data_cache().unwrap();
        hsr_scanner::data_cache::load_achievement_data(&references, false).unwrap()
    } else {
        load_embedded_gilore_reference().unwrap()
    };
    let mut decoder = HsrPacketDecoder::new(references.clone(), CaptureTargets::default()).unwrap();
    if let Some(directory) = option_path("--dump-dir") {
        decoder.set_packet_dump(directory);
    }
    let mut seen_errors = std::collections::BTreeSet::new();
    for path in files {
        let bytes = fs::read(&path).unwrap();
        let result = if raw {
            decoder.receive_packet(bytes)
        } else {
            decoder.receive_command(&bytes)
        };
        if let Err(error) = result {
            eprintln!(
                "{} DECODE-ERR {}",
                path.file_name().unwrap().to_string_lossy(),
                error
            );
        }
        let state = decoder.state();
        if let Some(error) = &state.last_transport_error {
            if seen_errors.insert(error.clone()) {
                eprintln!(
                    "{} TRANSPORT-ERR {error}",
                    path.file_name().unwrap().to_string_lossy()
                );
            }
        }
        if state.has_characters
            || state.has_light_cones
            || state.has_relics
            || state.has_achievements
        {
            eprintln!(
                "{} chars={}/{} lc={}/{} relics={}/{} ach={}/{} complete={}",
                path.file_name().unwrap().to_string_lossy(),
                state.has_characters,
                state.character_count,
                state.has_light_cones,
                state.light_cone_count,
                state.has_relics,
                state.relic_count,
                state.has_achievements,
                state.achievement_count,
                state.complete
            );
        }
    }
    let state = decoder.state();
    eprintln!(
        "transport packets={} commands={} keyExchangeCaptured={} lastError={}",
        state.packet_count,
        state.command_count,
        state.has_key_exchange,
        state.last_transport_error.as_deref().unwrap_or("none")
    );
    eprintln!(
        "FINAL chars={}/{} lc={}/{} relics={}/{} ach={}/{} complete={}",
        state.has_characters,
        state.character_count,
        state.has_light_cones,
        state.light_cone_count,
        state.has_relics,
        state.relic_count,
        state.has_achievements,
        state.achievement_count,
        state.complete
    );
    if !state.complete {
        std::process::exit(2);
    }
    fs::create_dir_all(&output).unwrap();
    let path = state
        .write_interop_exports(&output, &references, "")
        .unwrap();
    eprintln!("export={}", path.display());
}
