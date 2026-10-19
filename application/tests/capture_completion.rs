#![cfg(all(
    feature = "capture",
    feature = "test-as-invoker",
    target_os = "windows"
))]

use genshin_scanner::scanner::common::models::GoodExport;
use good_tools_app::gui::capture_tab::CaptureTabState;

#[test]
fn saved_capture_counts_match_progress_and_completion_writes_only_once() {
    let root = std::env::temp_dir().join(format!("capture-counts-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let mut capture = CaptureTabState::new(root.display().to_string());
    let character = serde_json::from_value(serde_json::json!({
        "key": "Amber", "level": 80, "constellation": 0, "ascension": 5,
        "talent": {"auto": 1, "skill": 1, "burst": 1}
    }))
    .unwrap();
    let weapon = serde_json::from_value(serde_json::json!({
        "key": "FavoniusWarbow", "level": 80, "ascension": 5,
        "refinement": 1, "location": "", "lock": false
    }))
    .unwrap();
    let artifact = serde_json::from_value(serde_json::json!({
        "setKey": "GladiatorsFinale", "slotKey": "flower", "level": 20,
        "rarity": 5, "mainStatKey": "hp", "substats": [],
        "location": "", "lock": false
    }))
    .unwrap();
    let mut export = GoodExport::new(
        Some(vec![character; 86]),
        Some(vec![weapon; 139]),
        Some(vec![artifact; 1269]),
    );
    export.achievements = Some(vec![80001, 80002]);
    let release = capture.inject_export_for_test(export);
    {
        let mut shared = capture.capture_state.lock().unwrap();
        // Raw inventory counts differ from the filtered export being saved.
        shared.character_count = 86;
        shared.weapon_count = 168;
        shared.artifact_count = 1487;
        shared.achievement_count = 1830;
    }
    capture.tick();
    assert!(capture.completed_export_path_for_test().is_none());
    assert_eq!(capture.capture_state.lock().unwrap().weapon_count, 168);
    release.send(()).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while capture.completed_export_path_for_test().is_none() {
        assert!(
            std::time::Instant::now() < deadline,
            "capture did not finish"
        );
        capture.tick();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let path = capture.completed_export_path_for_test().unwrap().to_owned();
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let shared = capture.capture_state.lock().unwrap();
    for (key, count) in [
        ("characters", shared.character_count),
        ("weapons", shared.weapon_count),
        ("artifacts", shared.artifact_count),
        ("achievements", shared.achievement_count),
    ] {
        assert_eq!(count, saved[key].as_array().unwrap().len(), "{key}");
    }
    drop(shared);
    for _ in 0..10 {
        capture.tick();
        assert!(!capture.is_busy());
        assert!(capture.handle.is_none());
        assert_eq!(
            capture.completed_export_path_for_test(),
            Some(path.as_str())
        );
    }
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(root).unwrap();
}
