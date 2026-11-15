use time::macros::{date, datetime};

use super::*;

fn entries(raw: &str) -> Vec<String> {
    raw.split(';').map(str::to_string).collect()
}

#[test]
fn parses_live_trading_hours() {
    // ES 202612 contract details, paper TWS, server 223, 2026-11-13.
    let sessions = parse_trading_sessions(&entries("20261112:1700-20261113:1600;20261114:CLOSED;20261115:1700-20261116:1600")).unwrap();

    assert_eq!(
        sessions,
        vec![
            TradingSession::Open {
                start: datetime!(2026-11-12 17:00),
                end: datetime!(2026-11-13 16:00),
            },
            TradingSession::Closed(date!(2026 - 11 - 14)),
            TradingSession::Open {
                start: datetime!(2026-11-15 17:00),
                end: datetime!(2026-11-16 16:00),
            },
        ]
    );
}

#[test]
fn skips_empty_entries() {
    let sessions = parse_trading_sessions(&entries("20261113:0830-20261113:1600;;")).unwrap();

    assert_eq!(
        sessions,
        vec![TradingSession::Open {
            start: datetime!(2026-11-13 08:30),
            end: datetime!(2026-11-13 16:00),
        }]
    );
    assert_eq!(parse_trading_sessions(&[]).unwrap(), vec![]);
}

#[test]
fn rejects_malformed_entries() {
    for raw in [
        "20090507:0700-1830",
        "20261113:0830",
        "2026111:0830-20261113:1600",
        "20261113:2500-20261113:1600",
        "20261332:CLOSED",
        "20261113:OPEN",
    ] {
        let err = parse_trading_sessions(&entries(raw)).unwrap_err();
        assert!(matches!(&err, Error::Parse(_, value, _) if value == raw), "{raw}: {err:?}");
    }
}
