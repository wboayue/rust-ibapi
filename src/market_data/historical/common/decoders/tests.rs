use crate::common::request_helpers::expect_proto;
use crate::common::test_utils::helpers::assert_decimal_parse_error;
use crate::messages::IncomingMessages;

use super::*;
use prost::Message;
use time::macros::{date, datetime};

use crate::market_data::historical::BarTimestamp;
use crate::testdata::builders::market_data::{
    histogram_data_response, histogram_entry, historical_data_daily_bar, historical_data_response, historical_tick_bid_ask, historical_tick_last,
    historical_tick_mid, historical_ticks_bid_ask_response, historical_ticks_last_response, historical_ticks_response,
};
use crate::testdata::builders::ResponseProtoEncoder;

// ---------------------------------------------------------------------------
// Happy-path proto decoders. Each test drives bytes through the `*_proto`
// helper directly (scanner precedent: src/scanner/common/decoders_tests.rs).
// ---------------------------------------------------------------------------

#[test]
fn test_decode_historical_data_proto() {
    let proto_msg = crate::proto::HistoricalData {
        req_id: Some(1),
        historical_data_bars: vec![
            crate::proto::HistoricalDataBar {
                date: Some("1681133400".into()),
                open: Some(185.50),
                high: Some(186.00),
                low: Some(185.00),
                close: Some(185.75),
                volume: Some("1000".into()),
                wap: Some("185.625".into()),
                bar_count: Some(150),
            },
            crate::proto::HistoricalDataBar {
                date: Some("20230411".into()),
                open: Some(186.00),
                high: Some(187.00),
                low: Some(185.50),
                close: Some(186.50),
                volume: Some("2000".into()),
                wap: Some("186.25".into()),
                bar_count: Some(300),
            },
        ],
    };

    let bars = decode_historical_data_proto(&proto_msg.encode_to_vec()).unwrap();
    assert_eq!(bars.len(), 2);

    assert_eq!(bars[0].date, BarTimestamp::DateTime(datetime!(2023-04-10 13:30:00 UTC)));
    assert_eq!(bars[0].open, 185.50);
    assert_eq!(bars[0].high, 186.00);
    assert_eq!(bars[0].low, 185.00);
    assert_eq!(bars[0].close, 185.75);
    assert_eq!(bars[0].volume, 1000.0);
    assert_eq!(bars[0].wap, 185.625);
    assert_eq!(bars[0].count, 150);

    assert_eq!(bars[1].open, 186.00);
    assert_eq!(bars[1].date, BarTimestamp::Date(date!(2023 - 04 - 11)));
    assert_eq!(bars[1].count, 300);
}

#[test]
fn test_decode_historical_ticks_last_proto() {
    let proto_msg = crate::proto::HistoricalTicksLast {
        req_id: Some(1),
        historical_ticks_last: vec![
            crate::proto::HistoricalTickLast {
                time: Some(1681133400),
                tick_attrib_last: Some(crate::proto::TickAttribLast {
                    past_limit: Some(true),
                    unreported: Some(false),
                }),
                price: Some(11.63),
                size: Some("100".into()),
                exchange: Some("ISLAND".into()),
                special_conditions: Some("O X".into()),
            },
            crate::proto::HistoricalTickLast {
                time: Some(1681133401),
                tick_attrib_last: Some(crate::proto::TickAttribLast {
                    past_limit: Some(false),
                    unreported: Some(true),
                }),
                price: Some(11.73),
                size: Some("50".into()),
                exchange: Some("FINRA".into()),
                special_conditions: Some("I".into()),
            },
        ],
        is_done: Some(true),
    };

    let (ticks, done) = decode_historical_ticks_last_proto(&proto_msg.encode_to_vec()).unwrap();
    assert!(done);
    assert_eq!(ticks.len(), 2);

    assert_eq!(ticks[0].timestamp, datetime!(2023-04-10 13:30:00 UTC));
    assert!(ticks[0].tick_attribute_last.past_limit);
    assert!(!ticks[0].tick_attribute_last.unreported);
    assert_eq!(ticks[0].price, 11.63);
    assert_eq!(ticks[0].size, Some(100.0));
    assert_eq!(ticks[0].exchange, "ISLAND");
    assert_eq!(ticks[0].special_conditions, "O X");

    assert_eq!(ticks[1].timestamp, datetime!(2023-04-10 13:30:01 UTC));
    assert!(!ticks[1].tick_attribute_last.past_limit);
    assert!(ticks[1].tick_attribute_last.unreported);
    assert_eq!(ticks[1].size, Some(50.0));
    assert_eq!(ticks[1].exchange, "FINRA");
}

#[test]
fn test_decode_head_timestamp_proto() {
    let proto_msg = crate::proto::HeadTimestamp {
        req_id: Some(1),
        head_timestamp: Some("1609459200".into()),
    };

    let result = decode_head_timestamp_proto(&proto_msg.encode_to_vec()).unwrap();
    assert_eq!(result, OffsetDateTime::from_unix_timestamp(1609459200).unwrap());
}

#[test]
fn test_decode_historical_ticks_proto() {
    let proto_msg = crate::proto::HistoricalTicks {
        req_id: Some(1),
        historical_ticks: vec![
            crate::proto::HistoricalTick {
                time: Some(1681133400),
                price: Some(150.0),
                size: Some("100".into()),
            },
            crate::proto::HistoricalTick {
                time: Some(1681133401),
                price: Some(150.5),
                size: Some("200".into()),
            },
        ],
        is_done: Some(false),
    };

    let (ticks, done) = decode_historical_ticks_proto(&proto_msg.encode_to_vec()).unwrap();
    assert_eq!(ticks.len(), 2);
    assert!(!done);
    assert_eq!(ticks[0].timestamp, datetime!(2023-04-10 13:30:00 UTC));
    assert_eq!(ticks[0].price, 150.0);
    assert_eq!(ticks[0].size, Some(100.0));
}

#[test]
fn test_decode_historical_ticks_bid_ask_proto() {
    let proto_msg = crate::proto::HistoricalTicksBidAsk {
        req_id: Some(1),
        historical_ticks_bid_ask: vec![crate::proto::HistoricalTickBidAsk {
            time: Some(1681133400),
            tick_attrib_bid_ask: Some(crate::proto::TickAttribBidAsk {
                bid_past_low: Some(true),
                ask_past_high: Some(false),
            }),
            price_bid: Some(149.0),
            price_ask: Some(151.0),
            size_bid: Some("100".into()),
            size_ask: Some("200".into()),
        }],
        is_done: Some(true),
    };

    let (ticks, done) = decode_historical_ticks_bid_ask_proto(&proto_msg.encode_to_vec()).unwrap();
    assert_eq!(ticks.len(), 1);
    assert!(done);
    assert_eq!(ticks[0].timestamp, datetime!(2023-04-10 13:30:00 UTC));
    assert!(ticks[0].tick_attribute_bid_ask.bid_past_low);
    assert!(!ticks[0].tick_attribute_bid_ask.ask_past_high);
    assert_eq!(ticks[0].price_bid, 149.0);
    assert_eq!(ticks[0].price_ask, 151.0);
    assert_eq!(ticks[0].size_bid, Some(100.0));
    assert_eq!(ticks[0].size_ask, Some(200.0));
}

#[test]
fn test_decode_histogram_data_proto() {
    let proto_msg = crate::proto::HistogramData {
        req_id: Some(1),
        histogram_data_entries: vec![
            crate::proto::HistogramDataEntry {
                price: Some(100.5),
                size: Some("50".into()),
            },
            crate::proto::HistogramDataEntry {
                price: Some(101.0),
                size: Some("75".into()),
            },
        ],
    };

    let result = decode_histogram_data_proto(&proto_msg.encode_to_vec()).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].price, 100.5);
    assert_eq!(result[0].size, Some(50.0));
    assert_eq!(result[1].price, 101.0);
    assert_eq!(result[1].size, Some(75.0));
}

#[test]
fn test_decode_historical_data_end_proto() {
    // Wire format for start/end uses "YYYYMMDD HH:MM:SS TZ".
    let proto_msg = crate::proto::HistoricalDataEnd {
        req_id: Some(1),
        start_date_str: Some("20260101 09:30:00 US/Eastern".into()),
        end_date_str: Some("20260105 16:00:00 US/Eastern".into()),
    };

    let (start, end) = decode_historical_data_end_proto(&proto_msg.encode_to_vec()).unwrap();
    assert!(start < end);
    assert_eq!(start.year(), 2026);
    assert_eq!(end.year(), 2026);
}

#[test]
fn test_decode_historical_schedule_proto() {
    let proto_msg = crate::proto::HistoricalSchedule {
        req_id: Some(1),
        start_date_time: Some("20260101-09:30:00".into()),
        end_date_time: Some("20260105-16:00:00".into()),
        time_zone: Some("US/Eastern".into()),
        historical_sessions: vec![crate::proto::HistoricalSession {
            start_date_time: Some("20260102-09:30:00".into()),
            end_date_time: Some("20260102-16:00:00".into()),
            ref_date: Some("20260102".into()),
        }],
    };

    let result = decode_historical_schedule_proto(&proto_msg.encode_to_vec()).unwrap();
    assert_eq!(result.time_zone, "US/Eastern");
    assert_eq!(result.sessions.len(), 1);
    assert_eq!(result.sessions[0].reference, date!(2026 - 01 - 02));
}

#[test]
fn test_decode_historical_schedule_unknown_timezone_errors() {
    // Gateway sends an unmappable timezone — must surface as Error::UnsupportedTimeZone.
    let proto_msg = crate::proto::HistoricalSchedule {
        req_id: Some(1),
        start_date_time: Some("20230414-09:30:00".into()),
        end_date_time: Some("20230414-16:00:00".into()),
        time_zone: Some("Bogus Standard Time".into()),
        historical_sessions: vec![],
    };

    let err = decode_historical_schedule_proto(&proto_msg.encode_to_vec()).expect_err("unknown tz must error");
    assert!(matches!(err, Error::UnsupportedTimeZone(ref name) if name == "Bogus Standard Time"));
    let rendered = err.to_string();
    assert!(rendered.contains("Bogus Standard Time"), "missing tz name: {rendered}");
    assert!(
        rendered.contains("register_timezone_alias"),
        "missing programmatic-fix pointer: {rendered}"
    );
    assert!(rendered.contains("IBAPI_TIMEZONE_ALIASES"), "missing env-var pointer: {rendered}");
}

#[test]
fn test_decode_historical_data_update_proto() {
    let proto_msg = crate::proto::HistoricalDataUpdate {
        req_id: Some(1),
        historical_data_bar: Some(crate::proto::HistoricalDataBar {
            date: Some("1681133400".into()), // unix timestamp
            open: Some(150.0),
            high: Some(151.0),
            low: Some(149.5),
            close: Some(150.75),
            volume: Some("1000".into()),
            wap: Some("150.5".into()),
            bar_count: Some(42),
        }),
    };

    let result = decode_historical_data_update_proto(&proto_msg.encode_to_vec()).unwrap();
    assert_eq!(result.open, 150.0);
    assert_eq!(result.high, 151.0);
    assert_eq!(result.low, 149.5);
    assert_eq!(result.close, 150.75);
    assert_eq!(result.volume, 1000.0);
    assert_eq!(result.wap, 150.5);
    assert_eq!(result.count, 42);
    assert_eq!(result.date, BarTimestamp::DateTime(datetime!(2023-04-10 13:30:00 UTC)));
}

#[test]
fn test_decode_historical_data_update_proto_missing_bar_defaults() {
    let proto_msg = crate::proto::HistoricalDataUpdate {
        req_id: Some(1),
        historical_data_bar: None,
    };

    let result = decode_historical_data_update_proto(&proto_msg.encode_to_vec()).unwrap();
    assert_eq!(result.count, 0);
    assert_eq!(result.date, BarTimestamp::DateTime(OffsetDateTime::UNIX_EPOCH));
}

// ---------------------------------------------------------------------------
// Public-wrapper guards. Each `decode_X(message)` rejects text framing with
// `Error::UnexpectedWireFormat` via `require_proto()` — scanner precedent #532.
// ---------------------------------------------------------------------------

fn text_message(payload: &str) -> ResponseMessage {
    ResponseMessage::from(payload)
}

#[test]
fn test_decode_head_timestamp_rejects_text_framing() {
    let message = text_message("88\09000\01560346200\0");
    let err = expect_proto(IncomingMessages::HeadTimestamp, decode_head_timestamp_proto)(&message).expect_err("text framing must be rejected");
    assert!(matches!(err, Error::UnexpectedWireFormat(_)), "got: {err:?}");
}

#[test]
fn test_decode_historical_data_rejects_text_framing() {
    let message = text_message("17\09000\01\020230413\0182.94\0186.50\0180.94\0185.90\0948837.22\0184.869\0324891\0");
    let err = decode_historical_data(&message).expect_err("text framing must be rejected");
    assert!(matches!(err, Error::UnexpectedWireFormat(_)), "got: {err:?}");
}

#[test]
fn test_decode_historical_data_end_rejects_text_framing() {
    let message = text_message("108\09000\020230315 09:30:00 UTC\020230315 10:30:00 UTC\0");
    let err = decode_historical_data_end(&message).expect_err("text framing must be rejected");
    assert!(matches!(err, Error::UnexpectedWireFormat(_)), "got: {err:?}");
}

#[test]
fn test_decode_historical_schedule_rejects_text_framing() {
    let message = text_message("106\09000\020230414-09:30:00\020230414-16:00:00\0US/Eastern\01\020230414-09:30:00\020230414-16:00:00\020230414\0");
    let err =
        expect_proto(IncomingMessages::HistoricalSchedule, decode_historical_schedule_proto)(&message).expect_err("text framing must be rejected");
    assert!(matches!(err, Error::UnexpectedWireFormat(_)), "got: {err:?}");
}

#[test]
fn test_decode_historical_data_update_rejects_text_framing() {
    let message = text_message("90\09000\0-1\01681133400\0185.50\0186.00\0185.00\0185.75\01000.5\0185.625\0150\0");
    let err = decode_historical_data_update(&message).expect_err("text framing must be rejected");
    assert!(matches!(err, Error::UnexpectedWireFormat(_)), "got: {err:?}");
}

#[test]
fn test_decode_historical_ticks_mid_point_rejects_text_framing() {
    let message = text_message("96\09000\01\01681133398\00\091.36\00\01\0");
    let err = decode_historical_ticks_mid_point(&message).expect_err("text framing must be rejected");
    assert!(matches!(err, Error::UnexpectedWireFormat(_)), "got: {err:?}");
}

#[test]
fn test_decode_historical_ticks_bid_ask_rejects_text_framing() {
    let message = text_message("97\09000\01\01681133399\00\011.63\011.83\02800\0100\01\0");
    let err = decode_historical_ticks_bid_ask(&message).expect_err("text framing must be rejected");
    assert!(matches!(err, Error::UnexpectedWireFormat(_)), "got: {err:?}");
}

#[test]
fn test_decode_historical_ticks_last_rejects_text_framing() {
    let message = text_message("98\09000\01\01681133400\00\011.63\024547\0ISLAND\0 O X\01\0");
    let err = decode_historical_ticks_last(&message).expect_err("text framing must be rejected");
    assert!(matches!(err, Error::UnexpectedWireFormat(_)), "got: {err:?}");
}

#[test]
fn test_decode_histogram_data_rejects_text_framing() {
    let message = text_message("89\09000\01\0125.50\01000\0");
    let err = expect_proto(IncomingMessages::HistogramData, decode_histogram_data_proto)(&message).expect_err("text framing must be rejected");
    assert!(matches!(err, Error::UnexpectedWireFormat(_)), "got: {err:?}");
}

// === decimal wire fields are routed through parse_optional_decimal (issue #716) ===
//
// Bar volume and wap are still plain f64; the tick and histogram sizes below
// are Option<f64>.

/// The builder stringifies an f64, so a malformed wire value needs raw proto.
fn historical_data_bytes(volume: &str) -> Vec<u8> {
    crate::proto::HistoricalData {
        req_id: Some(9000),
        historical_data_bars: vec![crate::proto::HistoricalDataBar {
            date: Some("20230101".into()),
            open: Some(100.0),
            high: Some(101.0),
            low: Some(99.0),
            close: Some(100.5),
            volume: Some(volume.into()),
            wap: Some("100.25".into()),
            bar_count: Some(10),
        }],
    }
    .encode_to_vec()
}

#[test]
fn test_decode_historical_data_proto_rejects_malformed_volume() {
    assert_decimal_parse_error(super::decode_historical_data_proto(&historical_data_bytes("abc")), "abc");
}

#[test]
fn test_decode_historical_data_proto_preserves_fractional_volume() {
    let bytes = historical_data_response()
        .bar(
            historical_data_daily_bar("20230101")
                .ohlc(100.0, 101.0, 99.0, 100.5)
                .volume(0.5)
                .wap(100.25)
                .count(10),
        )
        .encode_proto();

    let bars = super::decode_historical_data_proto(&bytes).unwrap();
    assert_eq!(bars[0].volume, 0.5);
}

#[test]
fn test_decode_historical_data_update_proto_rejects_malformed_volume() {
    let bytes = crate::proto::HistoricalDataUpdate {
        req_id: Some(9000),
        historical_data_bar: Some(crate::proto::HistoricalDataBar {
            date: Some("20230101".into()),
            volume: Some("abc".into()),
            ..Default::default()
        }),
    }
    .encode_to_vec();

    assert_decimal_parse_error(super::decode_historical_data_update_proto(&bytes), "abc");
}

// === tick and histogram sizes preserve fractional wire values (issue #716) ===
//
// These are the regression the issue reports: under the old `i32` typing a wire
// value of "0.5" failed `parse::<i32>()` and silently decoded as 0.

#[test]
fn test_decode_historical_ticks_proto_preserves_fractional_size() {
    let bytes = historical_ticks_response()
        .tick(historical_tick_mid(1_681_133_400, 150.0, 0.5))
        .encode_proto();

    let (ticks, _) = super::decode_historical_ticks_proto(&bytes).unwrap();
    assert_eq!(ticks[0].size, Some(0.5));
}

#[test]
fn test_decode_historical_ticks_last_proto_preserves_fractional_size() {
    let bytes = historical_ticks_last_response()
        .tick(historical_tick_last(1_681_133_400, 150.0, 0.25, "NYSE"))
        .encode_proto();

    let (ticks, _) = super::decode_historical_ticks_last_proto(&bytes).unwrap();
    assert_eq!(ticks[0].size, Some(0.25));
}

#[test]
fn test_decode_historical_ticks_bid_ask_proto_preserves_fractional_sizes() {
    let bytes = historical_ticks_bid_ask_response()
        .tick(historical_tick_bid_ask(1_681_133_400, 149.0, 151.0, 0.5, 0.75))
        .encode_proto();

    let (ticks, _) = super::decode_historical_ticks_bid_ask_proto(&bytes).unwrap();
    assert_eq!(ticks[0].size_bid, Some(0.5));
    assert_eq!(ticks[0].size_ask, Some(0.75));
}

#[test]
fn test_decode_histogram_data_proto_preserves_fractional_size() {
    let bytes = histogram_data_response().entry(histogram_entry(125.5, 0.5)).encode_proto();

    let entries = super::decode_histogram_data_proto(&bytes).unwrap();
    assert_eq!(entries[0].size, Some(0.5));
}

// --- malformed sizes ---

#[test]
fn test_decode_histogram_data_proto_rejects_malformed_size() {
    let bytes = histogram_data_response()
        .entry(histogram_entry(125.5, 0.0).size_wire(Some("abc")))
        .encode_proto();

    assert_decimal_parse_error(super::decode_histogram_data_proto(&bytes), "abc");
}

#[test]
fn test_decode_historical_ticks_proto_rejects_malformed_size() {
    let bytes = historical_ticks_response()
        .tick(historical_tick_mid(1_681_133_400, 150.0, 0.0).size_wire(Some("abc")))
        .encode_proto();

    assert_decimal_parse_error(super::decode_historical_ticks_proto(&bytes), "abc");
}
