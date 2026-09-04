use std::collections::HashMap;

use time::macros::datetime;
use time::{OffsetDateTime, PrimitiveDateTime};
use time_tz::{timezones, TimeZone};

use super::{find_timezone, map_timezone_name_with, parse_env_aliases, register_timezone_alias, resolve_local};

#[test]
fn test_find_timezone_standard() {
    assert!(!find_timezone("PST").is_empty());
    assert!(!find_timezone("America/New_York").is_empty());
}

#[test]
fn test_find_timezone_china_utf8() {
    let zones = find_timezone("中国标准时间");
    assert!(!zones.is_empty());
    assert_eq!(zones[0].name(), "Asia/Shanghai");

    let zones = find_timezone("北京时间");
    assert!(!zones.is_empty());
    assert_eq!(zones[0].name(), "Asia/Shanghai");
}

#[test]
fn test_find_timezone_china_english() {
    let zones = find_timezone("China Standard Time");
    assert!(!zones.is_empty());
    assert_eq!(zones[0].name(), "Asia/Shanghai");
}

#[test]
fn test_find_timezone_gmt() {
    let zones = find_timezone("Greenwich Mean Time");
    assert!(!zones.is_empty());
    assert_eq!(zones[0].name(), "Europe/London");

    let zones = find_timezone("GMT Standard Time");
    assert!(!zones.is_empty());
    assert_eq!(zones[0].name(), "Europe/London");

    let zones = find_timezone("British Summer Time");
    assert!(!zones.is_empty());
    assert_eq!(zones[0].name(), "Europe/London");
}

#[test]
fn test_find_timezone_mojibake() {
    // Simulate GB2312 decoded as UTF-8 lossy (contains replacement characters)
    let mojibake = "test\u{FFFD}\u{FFFD}zone";
    let zones = find_timezone(mojibake);
    assert!(!zones.is_empty());
    assert_eq!(zones[0].name(), "Asia/Shanghai");
}

#[test]
fn test_find_timezone_singapore() {
    let zones = find_timezone("SGT");
    assert!(!zones.is_empty());
    assert_eq!(zones[0].name(), "Asia/Singapore");
}

#[test]
fn test_find_timezone_european_continental() {
    let cases = [
        ("E. Europe Standard Time", "Europe/Bucharest"),
        ("Eastern European Standard Time", "Europe/Athens"),
        ("Eastern European Summer Time", "Europe/Athens"),
        ("FLE Standard Time", "Europe/Helsinki"),
        ("GTB Standard Time", "Europe/Athens"),
        ("Central European Standard Time", "Europe/Warsaw"),
        ("Central European Summer Time", "Europe/Warsaw"),
        ("W. Europe Standard Time", "Europe/Berlin"),
        ("Romance Standard Time", "Europe/Paris"),
    ];
    for (windows_name, expected_iana) in cases {
        let zones = find_timezone(windows_name);
        assert!(!zones.is_empty(), "no match for {windows_name}");
        assert_eq!(zones[0].name(), expected_iana, "wrong mapping for {windows_name}");
    }
}

#[test]
fn test_find_timezone_passthrough() {
    // Unknown timezone names pass through unchanged
    let zones = find_timezone("Unknown/Timezone");
    assert!(zones.is_empty());
}

#[test]
fn test_registry_overrides_builtin() {
    let mut reg = HashMap::new();
    reg.insert("China Standard Time".to_string(), "Europe/London".to_string());
    assert_eq!(map_timezone_name_with(&reg, "China Standard Time"), "Europe/London");
}

#[test]
fn test_registry_adds_new_alias() {
    let mut reg = HashMap::new();
    reg.insert("Made Up Time".to_string(), "Asia/Tokyo".to_string());
    assert_eq!(map_timezone_name_with(&reg, "Made Up Time"), "Asia/Tokyo");
}

#[test]
fn test_registry_falls_through_to_builtin() {
    let reg = HashMap::new();
    assert_eq!(map_timezone_name_with(&reg, "China Standard Time"), "Asia/Shanghai");
}

#[test]
fn test_registry_falls_through_to_mojibake() {
    let reg = HashMap::new();
    assert_eq!(map_timezone_name_with(&reg, "test\u{FFFD}\u{FFFD}"), "Asia/Shanghai");
}

#[test]
fn test_registry_passthrough_unknown() {
    let reg = HashMap::new();
    assert_eq!(map_timezone_name_with(&reg, "Some/Unknown"), "Some/Unknown");
}

#[test]
fn test_register_timezone_alias_smoke() {
    // Unique key avoids collision with other tests touching the registry.
    register_timezone_alias("__rust_ibapi_test_alias_xyz", "America/New_York");
    let zones = find_timezone("__rust_ibapi_test_alias_xyz");
    assert!(!zones.is_empty());
    assert_eq!(zones[0].name(), "America/New_York");
}

#[test]
fn test_parse_env_aliases_basic() {
    let pairs = parse_env_aliases("Foo=Asia/Tokyo;Bar=Europe/Berlin");
    assert_eq!(
        pairs,
        vec![
            ("Foo".to_string(), "Asia/Tokyo".to_string()),
            ("Bar".to_string(), "Europe/Berlin".to_string()),
        ]
    );
}

#[test]
fn test_parse_env_aliases_skips_malformed() {
    let pairs = parse_env_aliases("Foo=Asia/Tokyo;garbage;Bar=Europe/Berlin");
    assert_eq!(
        pairs,
        vec![
            ("Foo".to_string(), "Asia/Tokyo".to_string()),
            ("Bar".to_string(), "Europe/Berlin".to_string()),
        ]
    );
}

#[test]
fn test_parse_env_aliases_trims_whitespace() {
    let pairs = parse_env_aliases(" Foo Standard Time = Asia/Tokyo ; Bar = Europe/Berlin ");
    assert_eq!(
        pairs,
        vec![
            ("Foo Standard Time".to_string(), "Asia/Tokyo".to_string()),
            ("Bar".to_string(), "Europe/Berlin".to_string()),
        ]
    );
}

#[test]
fn test_parse_env_aliases_empty() {
    assert!(parse_env_aliases("").is_empty());
    assert!(parse_env_aliases(";;;").is_empty());
}

#[test]
fn test_parse_env_aliases_skips_empty_sides() {
    let pairs = parse_env_aliases("=Asia/Tokyo;Foo=;Bar=Europe/Berlin");
    assert_eq!(pairs, vec![("Bar".to_string(), "Europe/Berlin".to_string())]);
}

/// One policy for the two non-unique `assume_timezone` outcomes: a fold takes
/// the earlier occurrence, a gap is pushed forward by its length. Both DST
/// intervals are half-open in time-tz, so the gap's leading endpoint is the
/// transition instant, not a nonexistent time.
#[test]
fn test_resolve_local_dst_fold_and_gap_policy() {
    let eastern = timezones::db::america::NEW_YORK;
    let cases: &[(PrimitiveDateTime, OffsetDateTime, &str)] = &[
        (datetime!(2026-08-29 04:33:35), datetime!(2026-08-29 08:33:35 UTC), "unique reading, EDT"),
        // Spring forward 2026-03-08: 01:59:59 EST -> 03:00:00 EDT.
        (
            datetime!(2026-03-08 01:59:59),
            datetime!(2026-03-08 06:59:59 UTC),
            "last reading before the gap",
        ),
        (
            datetime!(2026-03-08 02:00:00),
            datetime!(2026-03-08 07:00:00 UTC),
            "gap leading endpoint is the transition instant",
        ),
        (
            datetime!(2026-03-08 02:00:01),
            datetime!(2026-03-08 07:00:01 UTC),
            "gap interior, pushed forward",
        ),
        (
            datetime!(2026-03-08 02:30:00),
            datetime!(2026-03-08 07:30:00 UTC),
            "gap interior, pushed forward to 03:30 EDT",
        ),
        (
            datetime!(2026-03-08 02:59:59),
            datetime!(2026-03-08 07:59:59 UTC),
            "gap trailing edge, pushed forward",
        ),
        (
            datetime!(2026-03-08 03:00:00),
            datetime!(2026-03-08 07:00:00 UTC),
            "first reading after the gap",
        ),
        // Fall back 2025-11-02: 01:59:59 EDT -> 01:00:00 EST; the fold takes EDT.
        (
            datetime!(2025-11-02 00:59:59),
            datetime!(2025-11-02 04:59:59 UTC),
            "last unique reading before the fold",
        ),
        (datetime!(2025-11-02 01:00:00), datetime!(2025-11-02 05:00:00 UTC), "fold start takes EDT"),
        (
            datetime!(2025-11-02 01:40:26),
            datetime!(2025-11-02 05:40:26 UTC),
            "#790 live input: fold interior takes EDT",
        ),
        (datetime!(2025-11-02 01:59:59), datetime!(2025-11-02 05:59:59 UTC), "fold end takes EDT"),
        (
            datetime!(2025-11-02 02:00:00),
            datetime!(2025-11-02 07:00:00 UTC),
            "first unique reading after the fold, EST",
        ),
    ];
    for (reading, expected, label) in cases {
        assert_eq!(resolve_local(*reading, eastern), *expected, "{label}: {reading}");
    }

    // A pushed-forward gap reading renders with the post-transition offset.
    let pushed = resolve_local(datetime!(2026-03-08 02:30:00), eastern);
    assert_eq!(pushed.offset().whole_hours(), -4, "rendered post-transition: {pushed}");
    assert_eq!(pushed.hour(), 3);
}

/// The push-forward is by the gap's actual length, not a fixed hour: Lord Howe
/// Island shifts by 30 minutes (02:00 +10:30 -> 02:30 +11:00 on 2025-10-05).
#[test]
fn test_resolve_local_half_hour_gap() {
    let lord_howe = timezones::db::australia::LORD_HOWE;
    let pushed = resolve_local(datetime!(2025-10-05 02:15:00), lord_howe);
    assert_eq!(pushed, datetime!(2025-10-04 15:45:00 UTC));
    assert_eq!(pushed.hour(), 2);
    assert_eq!(pushed.minute(), 45);
    assert_eq!(pushed.offset().whole_minutes(), 11 * 60);
}
