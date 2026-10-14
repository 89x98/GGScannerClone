use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::{Context, Result};

pub const SCANNER_EXPORT_PREFIX: &str = "good_export_";
pub const CAPTURE_EXPORT_PREFIX: &str = "genshin_export_";
pub const EXPORT_JSON_SUFFIX: &str = ".json";

/// Remove a file when present, while preserving any real filesystem failure.
///
/// Cache refreshers share this helper so a locked or unreadable cache cannot
/// be mistaken for a successful refresh.
pub(crate) fn remove_file_if_exists(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(error).with_context(|| format!("Failed to remove cached file: {}", path.display()))
        },
    }
}

/// Save a scan before applying retention. Partial scans always preserve history.
pub fn write_scan_export(
    output_dir: &Path,
    json: &str,
    timestamp: &str,
    partial: bool,
    only_latest: bool,
) -> Result<std::path::PathBuf> {
    write_export(
        output_dir,
        json,
        timestamp,
        partial,
        only_latest,
        SCANNER_EXPORT_PREFIX,
    )
}

/// Persist capture first, so a failed write never deletes the previous result.
pub fn write_capture_export(
    output_dir: &Path,
    json: &str,
    timestamp: &str,
    only_latest: bool,
) -> Result<std::path::PathBuf> {
    write_export(
        output_dir,
        json,
        timestamp,
        false,
        only_latest,
        CAPTURE_EXPORT_PREFIX,
    )
}

fn write_export(
    output_dir: &Path,
    json: &str,
    timestamp: &str,
    partial: bool,
    only_latest: bool,
    prefix: &str,
) -> Result<std::path::PathBuf> {
    use std::io::Write;
    std::fs::create_dir_all(output_dir)?;
    let path = output_dir.join(format!(
        "{}{}{}{}",
        prefix,
        if partial { "partial_" } else { "" },
        timestamp,
        EXPORT_JSON_SUFFIX
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    if let Err(error) = file
        .write_all(json.as_bytes())
        .and_then(|_| file.sync_all())
    {
        drop(file);
        let cleanup = std::fs::remove_file(&path);
        return Err(error).with_context(|| format!("Failed to save scan {}; incomplete-file cleanup={cleanup:?}; older exports retained", path.display()));
    }
    drop(file);
    if only_latest && !partial {
        remove_previous_exports_except(output_dir, prefix, Some(&path)).with_context(|| {
            format!(
                "New scan saved at {}; could not clean older exports",
                path.display()
            )
        })?;
    }
    Ok(path)
}

/// Delete previously generated timestamped exports that share `prefix`.
pub fn remove_previous_exports(output_dir: &Path, prefix: &str) -> Result<usize> {
    remove_previous_exports_except(output_dir, prefix, None)
}

fn remove_previous_exports_except(
    output_dir: &Path,
    prefix: &str,
    keep: Option<&Path>,
) -> Result<usize> {
    let mut removed = 0;
    for entry in fs::read_dir(output_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        if keep != Some(entry.path().as_path()) && is_generated_export_filename(file_name, prefix) {
            fs::remove_file(entry.path())?;
            removed += 1;
        }
    }
    Ok(removed)
}

pub fn is_generated_export_filename(file_name: &str, prefix: &str) -> bool {
    if !file_name.starts_with(prefix) || !file_name.ends_with(EXPORT_JSON_SUFFIX) {
        return false;
    }

    let timestamp = &file_name[prefix.len()..file_name.len() - EXPORT_JSON_SUFFIX.len()];
    is_export_timestamp(timestamp.strip_prefix("partial_").unwrap_or(timestamp))
}

fn is_export_timestamp(timestamp: &str) -> bool {
    let bytes = timestamp.as_bytes();
    if bytes.len() != 19 {
        return false;
    }
    for (idx, byte) in bytes.iter().enumerate() {
        let expected_separator = matches!(idx, 4 | 7 | 13 | 16);
        if expected_separator {
            if *byte != b'-' {
                return false;
            }
        } else if idx == 10 {
            if *byte != b'_' {
                return false;
            }
        } else if !byte.is_ascii_digit() {
            return false;
        }
    }

    let month = parse_two_digits(&bytes[5..7]);
    let day = parse_two_digits(&bytes[8..10]);
    let hour = parse_two_digits(&bytes[11..13]);
    let minute = parse_two_digits(&bytes[14..16]);
    let second = parse_two_digits(&bytes[17..19]);

    (1..=12).contains(&month)
        && (1..=31).contains(&day)
        && hour <= 23
        && minute <= 59
        && second <= 59
}

fn parse_two_digits(bytes: &[u8]) -> u8 {
    (bytes[0] - b'0') * 10 + (bytes[1] - b'0')
}

#[cfg(test)]
mod tests {
    use super::{is_generated_export_filename, CAPTURE_EXPORT_PREFIX, SCANNER_EXPORT_PREFIX};

    #[test]
    fn capture_save_failure_preserves_previous_export() {
        let dir = std::env::temp_dir().join(format!("capture-retention-{}", std::process::id()));
        let old =
            super::write_capture_export(&dir, "previous", "2026-10-13_13-45-09", true).unwrap();
        assert!(
            super::write_capture_export(&dir, "replacement", "2026-10-13_13-45-09", true).is_err()
        );
        assert_eq!(std::fs::read_to_string(&old).unwrap(), "previous");
        let new = super::write_capture_export(&dir, "new", "2026-10-14_13-45-09", true).unwrap();
        assert!(!old.exists());
        assert_eq!(std::fs::read_to_string(&new).unwrap(), "new");
        std::fs::remove_file(new).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn matches_generated_scanner_exports_only() {
        assert!(is_generated_export_filename(
            "good_export_2026-04-27_13-45-09.json",
            SCANNER_EXPORT_PREFIX
        ));
        assert!(!is_generated_export_filename(
            "good_export_2026-04-27_13-45.json",
            SCANNER_EXPORT_PREFIX
        ));
        assert!(!is_generated_export_filename(
            "good_export_latest.json",
            SCANNER_EXPORT_PREFIX
        ));
        assert!(!is_generated_export_filename(
            "good_export_2026-13-27_13-45-09.json",
            SCANNER_EXPORT_PREFIX
        ));
        assert!(!is_generated_export_filename(
            "genshin_export_2026-04-27_13-45-09.json",
            SCANNER_EXPORT_PREFIX
        ));
    }

    #[test]
    fn cleanup_keeps_the_new_export_and_removes_old_partial_results() {
        let dir = std::env::temp_dir().join(format!(
            "goodscanner-partial-cleanup-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let current = dir.join("good_export_2026-10-13_13-45-09.json");
        let partial = dir.join("good_export_partial_2026-10-12_13-45-09.json");
        let old = dir.join("good_export_2026-10-11_13-45-09.json");
        for path in [&current, &partial, &old] {
            std::fs::write(path, "{}").unwrap();
        }
        assert_eq!(
            super::remove_previous_exports_except(&dir, SCANNER_EXPORT_PREFIX, Some(&current))
                .unwrap(),
            2
        );
        assert!(current.exists());
        assert!(!partial.exists());
        assert!(!old.exists());
        std::fs::remove_file(current).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn partial_scan_preserves_complete_export_and_failed_write_preserves_both() {
        let dir =
            std::env::temp_dir().join(format!("goodscanner-save-partial-{}", std::process::id()));
        let full =
            super::write_scan_export(&dir, "complete", "2026-10-11_13-45-09", false, true).unwrap();
        let partial =
            super::write_scan_export(&dir, "partial", "2026-10-12_13-45-09", true, true).unwrap();
        assert_eq!(std::fs::read_to_string(&full).unwrap(), "complete");
        assert!(
            super::write_scan_export(&dir, "replacement", "2026-10-11_13-45-09", false, true)
                .is_err()
        );
        assert_eq!(std::fs::read_to_string(&full).unwrap(), "complete");
        assert_eq!(std::fs::read_to_string(&partial).unwrap(), "partial");
        let latest =
            super::write_scan_export(&dir, "latest", "2026-10-13_13-45-09", false, true).unwrap();
        assert!(!full.exists());
        assert!(!partial.exists());
        assert!(latest.exists());
        std::fs::remove_file(latest).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn matches_generated_capture_exports_only() {
        assert!(is_generated_export_filename(
            "genshin_export_2026-04-27_13-45-09.json",
            CAPTURE_EXPORT_PREFIX
        ));
        assert!(!is_generated_export_filename(
            "genshin_export_2026-04-27_13-45.json",
            CAPTURE_EXPORT_PREFIX
        ));
        assert!(!is_generated_export_filename(
            "genshin_export_latest.json",
            CAPTURE_EXPORT_PREFIX
        ));
        assert!(!is_generated_export_filename(
            "genshin_export_2026-13-27_13-45-09.json",
            CAPTURE_EXPORT_PREFIX
        ));
        assert!(!is_generated_export_filename(
            "good_export_2026-04-27_13-45-09.json",
            CAPTURE_EXPORT_PREFIX
        ));
    }

    #[test]
    fn remove_previous_exports_deletes_matching_files_only() {
        let dir =
            std::env::temp_dir().join(format!("goodscanner-export-cleanup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let keep_latest = dir.join("good_export_2026-04-27_13-45-09.json");
        let keep_unrelated = dir.join("genshin_export_2026-04-27_13-45-09.json");
        let keep_manual = dir.join("good_export_latest.json");
        std::fs::write(&keep_latest, "{}").unwrap();
        std::fs::write(&keep_unrelated, "{}").unwrap();
        std::fs::write(&keep_manual, "{}").unwrap();

        let removed = super::remove_previous_exports(&dir, SCANNER_EXPORT_PREFIX).unwrap();
        assert_eq!(removed, 1);
        assert!(!keep_latest.exists());
        assert!(keep_unrelated.exists());
        assert!(keep_manual.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
