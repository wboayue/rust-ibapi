//! Timezone utilities for handling IB Gateway timezone names.

use std::collections::HashMap;
use std::env;
use std::sync::{LazyLock, Mutex};

use log::{debug, warn};
use time_tz::{timezones, Tz};

/// Environment variable that seeds the timezone alias registry at startup.
/// Format: `name=iana;name=iana` (whitespace around tokens is trimmed).
const ENV_VAR: &str = "IBAPI_TIMEZONE_ALIASES";

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

/// Process-wide user-registered aliases. Seeded from `IBAPI_TIMEZONE_ALIASES`
/// on first access, then extended by `register_timezone_alias`. Entries here
/// take precedence over the built-in `TIMEZONE_ALIASES` table, allowing both
/// additions and overrides without rebuilding.
static TIMEZONE_REGISTRY: LazyLock<Mutex<HashMap<String, String>>> = LazyLock::new(|| Mutex::new(seed_from_env()));

/// Register a custom mapping from a gateway-supplied timezone name to an IANA
/// zone. Call before `Client::connect` for the mapping to apply during the
/// connection handshake.
///
/// User-registered aliases take precedence over the built-in mappings, so this
/// can be used to override a default if you disagree with it.
///
/// Equivalent runtime configuration is available via the
/// `IBAPI_TIMEZONE_ALIASES=name=iana;name=iana` environment variable, which
/// seeds the registry on first lookup.
///
/// # Example
/// ```no_run
/// ibapi::register_timezone_alias("Foo Standard Time", "Asia/Tokyo");
/// ```
pub fn register_timezone_alias(name: impl Into<String>, iana: impl Into<String>) {
    let name = name.into().trim().to_string();
    let iana = iana.into().trim().to_string();
    if name.is_empty() || iana.is_empty() {
        warn!("register_timezone_alias: ignoring empty name or iana value");
        return;
    }
    let mut registry = TIMEZONE_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    registry.insert(name, iana);
}

/// Find timezone by name, handling non-standard names from IB Gateway.
///
/// Lookup precedence (highest to lowest):
/// 1. User-registered aliases (`register_timezone_alias` and `IBAPI_TIMEZONE_ALIASES`)
/// 2. Built-in `TIMEZONE_ALIASES` table
/// 3. Mojibake heuristic (GB2312/GBK decoded as UTF-8 lossy → `Asia/Shanghai`)
/// 4. Passthrough to `time_tz` (handles IANA names and abbreviations)
pub fn find_timezone(name: &str) -> Vec<&'static Tz> {
    let mapped = map_timezone_name(name);
    timezones::find_by_name(&mapped)
}

fn map_timezone_name(name: &str) -> String {
    let registry = TIMEZONE_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    map_timezone_name_with(&registry, name)
}

/// Pure mapping function used by `map_timezone_name` and unit tests. Tests
/// pass a fresh `HashMap` so they don't pollute the process-wide registry.
fn map_timezone_name_with(registry: &HashMap<String, String>, name: &str) -> String {
    if let Some(iana) = registry.get(name) {
        debug!("timezone alias matched (registry): {name:?} -> {iana:?}");
        return iana.clone();
    }

    for &(alias, iana) in TIMEZONE_ALIASES {
        if name == alias {
            return iana.to_string();
        }
    }

    // GB2312/GBK encoded strings decoded as UTF-8 lossy contain U+FFFD.
    // In IB Gateway context, this indicates a Chinese installation.
    if name.contains('\u{FFFD}') {
        return "Asia/Shanghai".to_string();
    }

    name.to_string()
}

fn seed_from_env() -> HashMap<String, String> {
    match env::var(ENV_VAR) {
        Ok(raw) => parse_env_aliases(&raw).into_iter().collect(),
        Err(_) => HashMap::new(),
    }
}

fn parse_env_aliases(raw: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for entry in raw.split(';') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        match entry.split_once('=') {
            Some((name, iana)) => {
                let name = name.trim();
                let iana = iana.trim();
                if name.is_empty() || iana.is_empty() {
                    warn!("ignoring malformed {ENV_VAR} entry: {entry:?}");
                    continue;
                }
                out.push((name.to_string(), iana.to_string()));
            }
            None => {
                warn!("ignoring malformed {ENV_VAR} entry (missing '='): {entry:?}");
            }
        }
    }
    out
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
}
