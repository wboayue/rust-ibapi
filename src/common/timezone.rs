//! Timezone utilities for handling IB Gateway timezone names

use time_tz::{timezones, Tz};

/// Non-standard timezone names sent by IB Gateway mapped to IANA identifiers.
const TIMEZONE_ALIASES: &[(&str, &str)] = &[
    // Chinese
    ("中国标准时间", "Asia/Shanghai"),
    ("北京时间", "Asia/Shanghai"),
    // Windows English
    ("China Standard Time", "Asia/Shanghai"),
    ("Greenwich Mean Time", "Europe/London"),
    ("GMT Standard Time", "Europe/London"),
    ("British Summer Time", "Europe/London"),
    // Southeast Asia
    ("SGT", "Asia/Singapore"),
    // European continental (Windows names sent by IB Gateway on non-English Windows)
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

/// Find timezone by name, handling non-standard names from IB Gateway.
///
/// IB Gateway may send timezone names in various formats:
/// - IANA names: "America/New_York", "Asia/Shanghai"
/// - Abbreviations: "PST", "EST"
/// - Windows names: "China Standard Time", "Greenwich Mean Time"
/// - Localized names: "中国标准时间" (Chinese)
/// - Mojibake from encoding issues (GB2312 decoded as UTF-8)
pub fn find_timezone(name: &str) -> Vec<&'static Tz> {
    let mapped = map_timezone_name(name);
    timezones::find_by_name(mapped)
}

/// Map non-standard timezone names to IANA identifiers.
fn map_timezone_name(name: &str) -> &str {
    for &(alias, iana) in TIMEZONE_ALIASES {
        if name == alias {
            return iana;
        }
    }

    // GB2312/GBK encoded strings decoded as UTF-8 lossy contain U+FFFD.
    // In IB Gateway context, this indicates a Chinese installation.
    if name.contains('\u{FFFD}') {
        return "Asia/Shanghai";
    }

    name
}

#[cfg(test)]
mod tests {
    use super::*;
    use time_tz::TimeZone;

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
    fn test_find_timezone_singapore() {
        let zones = find_timezone("SGT");
        assert!(!zones.is_empty());
        assert_eq!(zones[0].name(), "Asia/Singapore");
    }

    #[test]
    fn test_find_timezone_passthrough() {
        // Unknown timezone names pass through unchanged
        let zones = find_timezone("Unknown/Timezone");
        assert!(zones.is_empty());
    }
}
