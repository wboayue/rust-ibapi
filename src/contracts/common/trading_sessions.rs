//! Parsing of `ContractDetails::trading_hours` and `liquid_hours`.

use time::macros::format_description;
use time::{Date, PrimitiveDateTime};

use crate::contracts::TradingSession;
use crate::Error;

const SESSION_FORMAT: &str = "expected YYYYMMDD:HHMM-YYYYMMDD:HHMM or YYYYMMDD:CLOSED";

/// Parses the session entries TWS sends in `tradingHours` / `liquidHours`, one per
/// `;`-separated item. Empty items are skipped. The times stay wall clock in the
/// product's `time_zone_id`: no zone is resolved.
pub(crate) fn parse_trading_sessions(entries: &[String]) -> Result<Vec<TradingSession>, Error> {
    entries
        .iter()
        .map(|entry| entry.trim())
        .filter(|entry| !entry.is_empty())
        .map(parse_trading_session)
        .collect()
}

fn parse_trading_session(entry: &str) -> Result<TradingSession, Error> {
    if let Some(date) = entry.strip_suffix(":CLOSED") {
        let date = Date::parse(date, format_description!("[year][month][day]")).map_err(|_| Error::parse_field(entry, SESSION_FORMAT))?;
        return Ok(TradingSession::Closed(date));
    }

    let (start, end) = entry.split_once('-').ok_or_else(|| Error::parse_field(entry, SESSION_FORMAT))?;
    let parse = |value: &str| {
        PrimitiveDateTime::parse(value, format_description!("[year][month][day]:[hour][minute]"))
            .map_err(|_| Error::parse_field(entry, SESSION_FORMAT))
    };
    Ok(TradingSession::Open {
        start: parse(start)?,
        end: parse(end)?,
    })
}

#[cfg(test)]
#[path = "trading_sessions_tests.rs"]
mod tests;
