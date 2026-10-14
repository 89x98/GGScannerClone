use genshin_scanner::scanner::common::{
    annotator, capture_frame::CaptureFrame, coord_scaler::CoordScaler, pixel_utils,
};
use image::{Rgb, RgbImage};

// A separate integration-test process keeps the global dump setting isolated
// from scanner unit tests, which intentionally initialize it with dumping off.
#[test]
fn observation_boundaries_capture_retries_errors_and_standalone_checks_once() {
    let previous_cwd = std::env::current_dir().unwrap();
    let test_root =
        std::env::temp_dir().join(format!("scanner_observer_test_{}", std::process::id()));
    std::fs::create_dir_all(&test_root).unwrap();
    std::env::set_current_dir(&test_root).unwrap();
    let image = RgbImage::from_pixel(11, 7, Rgb([3, 4, 5]));
    annotator::init_for("hsr", true);
    let root = std::path::Path::new("debug_images");
    annotator::begin_item_for("hsr", "observation_test", 0, &CoordScaler::new(1920, 1080));
    for text in ["", "name"] {
        let result: Result<String, &str> = annotator::observe_ocr("name", &image, || {
            annotator::observe_ocr("nested_model", &image, || Ok(text.into()))
        });
        assert_eq!(result.unwrap(), text);
    }
    let failed: Result<String, &str> =
        annotator::observe_ocr("level", &image, || Err("inference failed"));
    assert!(failed.is_err());
    annotator::observe_detection("lock", &image, || (Some(false), 1.0));
    annotator::finalize_error(None, "panel failed");
    // A domain check before any item context must still be captured.
    annotator::observe_detection("standalone_lock", &image, || false);
    annotator::flush();
    let item = root.join("hsr_observation_test/0000");
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(item.join("ocr_fields.json")).unwrap())
            .unwrap();
    let fields = manifest["fields"].as_array().unwrap();
    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0]["field"], "name");
    assert_eq!(fields[1]["crop"], "name_2.png");
    assert_eq!(fields[2]["inference_error"], true);
    assert_eq!(manifest["detections"][0]["result"], "(Some(false), 1.0)");
    for field in fields {
        assert_eq!(
            image::open(item.join(field["crop"].as_str().unwrap()))
                .unwrap()
                .to_rgb8(),
            image
        );
    }
    let standalone = root.join("hsr_observation_standalone_lock/0000");
    assert!(standalone.join("standalone_lock.png").exists());
    assert!(!standalone.join("full.png").exists());
    annotator::observe_detection("standalone_lock", &image, || true);
    annotator::flush();
    assert!(root
        .join("hsr_observation_standalone_lock/0001/standalone_lock.png")
        .exists());

    // The manager checks rarity on the control thread before dispatching OCR.
    // Hundreds of pixel reads must produce one semantic crop, not hundreds
    // of folders/threads or searches through a previous dump library.
    let scaler = CoordScaler::new(1920, 1080);
    let mut panel = RgbImage::from_pixel(1920, 1080, Rgb([0, 0, 0]));
    for y in 370..=374 {
        for x in 1340..=1500 {
            panel.put_pixel(x, y, Rgb([200, 180, 50]));
        }
    }
    let frame = CaptureFrame::full(panel);
    for _ in 0..3 {
        assert_eq!(
            pixel_utils::detect_artifact_rarity_evidence(&frame, &scaler),
            Some(5)
        );
    }
    annotator::flush();
    assert!(!root.join("hsr_observation_frame_pixel").exists());
    let rarity_root = root.join("hsr_observation_rarity_stars");
    assert_eq!(std::fs::read_dir(&rarity_root).unwrap().count(), 3);
    let crop = image::open(rarity_root.join("0000/rarity_stars.png"))
        .unwrap()
        .to_rgb8();
    assert_eq!(crop.dimensions(), (161, 5));
    assert!(crop.pixels().all(|pixel| pixel.0 == [200, 180, 50]));
    // Reusing an item index writes to the existing path rather than creating
    // a suffixed folder, and initialization resets standalone indices.
    annotator::begin_item_for("hsr", "observation_test", 0, &scaler);
    annotator::observe_detection("replacement", &image, || true);
    annotator::finalize_success("{}");
    annotator::flush();
    assert_eq!(
        std::fs::read_dir(root.join("hsr_observation_test"))
            .unwrap()
            .count(),
        1
    );
    let replacement: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(item.join("ocr_fields.json")).unwrap())
            .unwrap();
    assert_eq!(replacement["detections"][0]["field"], "replacement");
    annotator::init_for("hsr", true);
    annotator::observe_detection("standalone_lock", &image, || false);
    annotator::flush();
    assert_eq!(
        std::fs::read_dir(root.join("hsr_observation_standalone_lock"))
            .unwrap()
            .count(),
        1
    );
    annotator::init(false);
    let disabled: Result<String, &str> =
        annotator::observe_ocr("disabled", &image, || Ok("text".into()));
    assert_eq!(disabled.unwrap(), "text");
    assert!(!root.join("gi_observation_disabled").exists());
    std::env::set_current_dir(previous_cwd).unwrap();
    std::fs::remove_dir_all(test_root).unwrap();
}
