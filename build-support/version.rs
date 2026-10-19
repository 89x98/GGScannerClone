use time::{Date, OffsetDateTime};

/// Keep CI's CalVer/revision; ordinary Cargo versions identify local builds.
pub fn version_at(package_version: &str, today: Date) -> String {
    let major = package_version
        .split('.')
        .next()
        .and_then(|major| major.parse::<u32>().ok())
        .expect("Cargo package version must have a numeric major component");
    if major >= 20_000_000 {
        return package_version.to_owned();
    }

    let yesterday = today
        .previous_day()
        .expect("build date must have a previous day");
    format!(
        "{}{:02}{:02}.0.0",
        yesterday.year(),
        u8::from(yesterday.month()),
        yesterday.day()
    )
}

pub fn configure() -> String {
    // This path deliberately does not exist: Cargo must refresh the date on
    // every build, including after midnight when no source files changed.
    let out_dir = std::env::var("OUT_DIR").expect("Cargo must provide its output directory");
    println!("cargo:rerun-if-changed={out_dir}/refresh-build-date");
    let version = version_at(
        &std::env::var("CARGO_PKG_VERSION").expect("Cargo must provide its package version"),
        OffsetDateTime::now_utc().date(),
    );
    println!("cargo:rustc-env=GGSCANNER_BUILD_VERSION={version}");
    version
}
