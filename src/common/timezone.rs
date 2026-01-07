//! Timezone utilities for handling IB Gateway timezone names

use time_tz::{timezones, Tz};

/// Find timezone by name, handling non-standard names from IB Gateway.
///
/// IB Gateway may send timezone names in various formats:
/// - IANA names: "America/New_York", "Asia/Shanghai"
/// - Abbreviations: "PST", "EST"
/// - Windows names: "China Standard Time"
/// - Localized names: "中国标准时间" (Chinese)
/// - Mojibake from encoding issues (GB2312 decoded as UTF-8)
pub fn find_timezone(name: &str) -> Vec<&'static Tz> {
    let mapped = map_timezone_name(name);
    timezones::find_by_name(mapped)
}

/// Map non-standard timezone names to IANA identifiers.
fn map_timezone_name(name: &str) -> &str {
    // UTF-8 Chinese timezone names
    if name == "中国标准时间" || name == "北京时间" {
        return "Asia/Shanghai";
    }

    // Windows English timezone name
    if name == "China Standard Time" {
        return "Asia/Shanghai";
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
    fn test_find_timezone_mojibake() {
        // Simulate GB2312 decoded as UTF-8 lossy (contains replacement characters)
        let mojibake = "test\u{FFFD}\u{FFFD}zone";
        let zones = find_timezone(mojibake);
        assert!(!zones.is_empty());
        assert_eq!(zones[0].name(), "Asia/Shanghai");
    }

    #[test]
    fn test_find_timezone_passthrough() {
        // Unknown timezone names pass through unchanged
        let zones = find_timezone("Unknown/Timezone");
        assert!(zones.is_empty());
    }
}
