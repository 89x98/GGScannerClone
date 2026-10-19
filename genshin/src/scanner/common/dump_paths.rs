//! Flat game-prefixed dump categories with reusable overwrite paths.
use std::path::{Path, PathBuf};

pub fn component(name: &str) -> String {
    // Windows paths are case insensitive. Prefixing also avoids device names.
    let name: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || "_-[]".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let name = name.trim_matches('_');
    if name.is_empty() {
        return "unknown".into();
    }
    let upper = name.to_ascii_uppercase();
    if matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (upper.len() == 4
            && (upper.starts_with("COM") || upper.starts_with("LPT"))
            && upper.as_bytes()[3].is_ascii_digit())
    {
        format!("_{name}")
    } else {
        name.into()
    }
}

fn prefix(game: &str) -> &'static str {
    match game {
        "genshin" => "gi_",
        "hsr" => "hsr_",
        _ => panic!("unknown dump game: {game}"),
    }
}

/// Replace this game's previous dump library before starting any workers.
/// The caller must flush pending writes before invoking this function.
pub fn start_run(base: &str, game: &str) {
    let prefix = prefix(game);
    std::fs::create_dir_all(base).expect("cannot create debug image directory");
    let root = std::fs::canonicalize(base).expect("cannot resolve debug image directory");
    for entry in std::fs::read_dir(&root).expect("cannot list debug image directory") {
        let entry = entry.expect("cannot read debug image directory entry");
        if !entry.file_name().to_string_lossy().starts_with(prefix) {
            continue;
        }
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path).expect("cannot inspect dump category");
        // Refuse links/junctions: cleanup must stay inside the chosen dump root.
        assert!(!metadata.file_type().is_symlink(), "linked dump category");
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            assert_eq!(
                metadata.file_attributes() & 0x400,
                0,
                "linked dump category"
            );
        }
        if metadata.is_dir() {
            let resolved = std::fs::canonicalize(&path).expect("cannot resolve dump category");
            assert_eq!(resolved.parent(), Some(root.as_path()));
            std::fs::remove_dir_all(resolved).expect("cannot clear previous dump category");
        }
    }
}

pub fn save_image(dir: &Path, name: &str, image: &image::RgbImage) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    image
        .save(dir.join(format!("{}.png", component(stem))))
        .map_err(std::io::Error::other)
}

pub fn category_dir(base: &str, game: &str, category: &str) -> PathBuf {
    // Category names are code-owned identifiers. Reject aliases instead of
    // sanitizing two different owners into the same Windows directory.
    assert!(!category.is_empty());
    assert!(category
        .bytes()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_'));
    let dir = Path::new(base).join(format!("{}{category}", prefix(game)));
    std::fs::create_dir_all(&dir).expect("cannot create debug image category directory");
    dir
}

pub fn item_dir(base: &str, game: &str, category: &str, index: usize) -> PathBuf {
    let dir = category_dir(base, game, category).join(format!("{index:04}"));
    std::fs::create_dir_all(&dir).expect("cannot create debug image item directory");
    dir
}

/// Reserve a filename without overwriting another capture or metadata file.
pub fn unique_name(
    used: &mut std::collections::HashSet<String>,
    stem: &str,
    extension: &str,
) -> String {
    let stem = component(stem);
    for attempt in 1.. {
        let name = if attempt == 1 {
            format!("{stem}.{extension}")
        } else {
            format!("{stem}_{attempt}.{extension}")
        };
        if used.insert(name.to_lowercase()) {
            return name;
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_scan_replaces_own_categories_without_touching_other_game() {
        let base = std::env::temp_dir()
            .join(format!("scanner_paths_test_{}", std::process::id()))
            .to_string_lossy()
            .into_owned();
        start_run(&base, "genshin");
        start_run(&base, "hsr");
        let paths: Vec<_> = std::thread::scope(|scope| {
            let jobs: Vec<_> = (0..16)
                .map(|i| {
                    let base = &base;
                    scope.spawn(move || {
                        item_dir(
                            base,
                            if i % 2 == 0 { "genshin" } else { "hsr" },
                            "characters",
                            0,
                        )
                    })
                })
                .collect();
            jobs.into_iter().map(|job| job.join().unwrap()).collect()
        });
        assert_eq!(
            paths.iter().collect::<std::collections::HashSet<_>>().len(),
            2
        );
        for path in &paths {
            std::fs::write(path.join("sentinel"), "keep").unwrap();
        }
        let stale = item_dir(&base, "genshin", "characters", 99);
        let other_category = item_dir(&base, "genshin", "weapons", 4);
        let unrelated = Path::new(&base).join("characters");
        std::fs::create_dir_all(&unrelated).unwrap();
        start_run(&base, "genshin");
        let next = item_dir(&base, "genshin", "characters", 0);
        assert_eq!(next, Path::new(&base).join("gi_characters/0000"));
        assert!(!next.join("sentinel").exists());
        assert!(!stale.exists());
        assert!(!other_category.exists());
        assert!(unrelated.exists());
        for path in paths {
            if path.parent().unwrap().ends_with("hsr_characters") {
                assert_eq!(
                    std::fs::read_to_string(path.join("sentinel")).unwrap(),
                    "keep"
                );
            } else {
                assert!(!path.join("sentinel").exists());
            }
        }
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    #[should_panic]
    fn category_aliases_are_rejected() {
        category_dir("unused", "genshin", "Characters");
    }

    #[test]
    fn image_writes_overwrite_the_requested_path() {
        let base = std::env::temp_dir().join(format!("scanner_legacy_test_{}", std::process::id()));
        let first = image::RgbImage::from_pixel(3, 2, image::Rgb([1, 2, 3]));
        let second = image::RgbImage::from_pixel(3, 2, image::Rgb([4, 5, 6]));
        save_image(&base, "field.png", &first).unwrap();
        save_image(&base, "field.png", &second).unwrap();
        assert_eq!(
            image::open(base.join("field.png")).unwrap().to_rgb8(),
            second
        );
        assert!(!base.join("field_2.png").exists());
        assert_eq!(component("../CON"), "_CON");
        std::fs::remove_dir_all(base).unwrap();
    }
}
