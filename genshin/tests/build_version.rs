#[allow(dead_code)]
#[path = "../../build-support/version.rs"]
mod version;

use time::{Date, Month};

#[test]
fn local_builds_use_yesterday_with_revision_zero() {
    for (year, month, day, expected) in [
        (2026, Month::October, 9, "20261008.0.0"),
        (2026, Month::January, 1, "20251231.0.0"),
        (2024, Month::March, 1, "20240229.0.0"),
        (2026, Month::March, 1, "20260228.0.0"),
    ] {
        let today = Date::from_calendar_date(year, month, day).unwrap();
        assert_eq!(version::version_at("0.2.0", today), expected);
    }
}

#[test]
fn official_builds_keep_the_release_date_and_revision() {
    let today = Date::from_calendar_date(2026, Month::October, 9).unwrap();
    assert_eq!(
        version::version_at("20261009.125.0", today),
        "20261009.125.0"
    );
}
