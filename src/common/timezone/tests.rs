use std::collections::HashMap;

use time_tz::TimeZone;

use super::{find_timezone, map_timezone_name_with, parse_env_aliases, register_timezone_alias};

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
