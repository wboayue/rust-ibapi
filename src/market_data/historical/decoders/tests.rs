use time::macros::{date, datetime};
use time_tz;

use super::*;

#[test]
fn test_decode_head_timestamp() {
    let mut message = ResponseMessage::from("88\09000\01560346200\0");

    let head_timestamp = super::decode_head_timestamp(&mut message).expect("error decoding trade tick");

    assert_eq!(head_timestamp, datetime!(2019-06-12 13:30).assume_utc(), "head_timestamp");
}

#[test]
fn test_decode_historical_schedule() {
    let time_zone: &Tz = time_tz::timezones::db::america::NEW_YORK;

    let mut message =
        ResponseMessage::from("106\09000\020230414-09:30:00\020230414-16:00:00\0US/Eastern\01\020230414-09:30:00\020230414-16:00:00\020230414\0");

    let schedule = decode_historical_schedule(&mut message).expect("error decoding historical schedule");

    assert_eq!(
        schedule.start,
        datetime!(2023-04-14 9:30:00).assume_timezone(time_zone).unwrap(),
        "schedule.start"
    );
    assert_eq!(
        schedule.end,
        datetime!(2023-04-14 16:00:00).assume_timezone(time_zone).unwrap(),
        "schedule.end"
    );
    assert_eq!(schedule.time_zone, "US/Eastern", "schedule.time_zone");

    assert_eq!(schedule.sessions.len(), 1, "schedule.sessions.len()");
    assert_eq!(schedule.sessions[0].reference, date!(2023 - 04 - 14), "schedule.sessions[0].reference");
    assert_eq!(
        schedule.sessions[0].start,
        datetime!(2023-04-14 9:30:00).assume_timezone(time_zone).unwrap(),
        "schedule.sessions[0].start"
    );
    assert_eq!(
        schedule.sessions[0].end,
        datetime!(2023-04-14 16:00:00.0).assume_timezone(time_zone).unwrap(),
        "schedule.sessions[0].end"
    );
}

#[test]
fn test_decode_historical_data() {
    let mut message = ResponseMessage::from("17\09000\020230413  16:31:22\020230415  16:31:22\02\020230413\0182.9400\0186.5000\0180.9400\0185.9000\0948837.22\0184.869\0324891\020230414\0183.8800\0186.2800\0182.0100\0185.0000\0810998.27\0183.9865\0277547\0");

    let server_version = server_versions::HISTORICAL_SCHEDULE;
    let time_zone: &Tz = time_tz::timezones::db::america::NEW_YORK;

    let historical_data = decode_historical_data(server_version, time_zone, &mut message).expect("error decoding historical data");

    assert_eq!(
        historical_data.start,
        datetime!(2023-04-13 16:31:22).assume_timezone(time_zone).unwrap(),
        "historical_data.start"
    );
    assert_eq!(
        historical_data.end,
        datetime!(2023-04-15 16:31:22).assume_timezone(time_zone).unwrap(),
        "historical_data.end"
    );

    assert_eq!(historical_data.bars.len(), 2, "historical_data.bars.len()");
    assert_eq!(
        historical_data.bars[0].date,
        datetime!(2023-04-13 0:00:00 UTC),
        "historical_data.bars[0].date"
    );
    assert_eq!(historical_data.bars[0].open, 182.94, "historical_data.bars[0].open");
    assert_eq!(historical_data.bars[0].high, 186.50, "historical_data.bars[0].high");
    assert_eq!(historical_data.bars[0].low, 180.94, "historical_data.bars[0].low");
    assert_eq!(historical_data.bars[0].close, 185.90, "historical_data.bars[0].close");
    assert_eq!(historical_data.bars[0].volume, 948837.22, "historical_data.bars[0].volume");
    assert_eq!(historical_data.bars[0].wap, 184.869, "historical_data.bars[0].wap");
    assert_eq!(historical_data.bars[0].count, 324891, "historical_data.bars[0].count");
}

#[test]
fn test_decode_historical_tick_bid_ask() {
    let sample_message = "97\09000\04\01681133399\00\011.63\011.83\02800\0100\01681133400\00\011.63\011.83\02800\0200\01681133400\00\011.63\011.72\02800\0100\01681133400\00\011.63\011.83\02800\0200\01\0";
    let mut message = ResponseMessage::from(sample_message);

    let (ticks, done) = decode_historical_ticks_bid_ask(&mut message).unwrap();

    assert_eq!(ticks.len(), 4, "ticks.len()");
    assert_eq!(done, true, "done");

    assert_eq!(ticks[0].timestamp, datetime!(2023-04-10 13:29:59 UTC), "ticks[0].timestamp");
    assert_eq!(
        ticks[0].tick_attribute_bid_ask,
        TickAttributeBidAsk {
            bid_past_low: false,
            ask_past_high: false
        },
        "ticks[0].tick_attribute_bid_ask"
    );
    assert_eq!(ticks[0].price_bid, 11.63, "ticks[0].price_bid");
    assert_eq!(ticks[0].price_ask, 11.83, "ticks[0].price_ask");
    assert_eq!(ticks[0].size_bid, 2800, "ticks[0].size_bid");
    assert_eq!(ticks[0].size_ask, 100, "ticks[0].size_ask");

    assert_eq!(ticks[3].timestamp, datetime!(2023-04-10 13:30:00 UTC), "ticks[0].timestamp");
    assert_eq!(
        ticks[3].tick_attribute_bid_ask,
        TickAttributeBidAsk {
            bid_past_low: false,
            ask_past_high: false
        },
        "ticks[0].tick_attribute_bid_ask"
    );
    assert_eq!(ticks[3].price_bid, 11.63, "ticks[0].price_bid");
    assert_eq!(ticks[3].price_ask, 11.83, "ticks[0].price_ask");
    assert_eq!(ticks[3].size_bid, 2800, "ticks[0].size_bid");
    assert_eq!(ticks[3].size_ask, 200, "ticks[0].size_ask");
}

#[test]
fn test_decode_historical_tick_last() {
    let sample_message = "98\09000\07\01681133400\00\011.63\024547\0ISLAND\0 O X\01681133400\02\011.73\01\0DRCTEDGE\0   I\01681133401\00\011.63\0179\0FINRA\0\01681133401\02\011.73\01\0FINRA\0   I\01681133402\02\011.63\01\0FINRA\0 4 I\01681133402\02\011.73\01\0FINRA\0   I\01681133402\02\011.73\01\0FINRA\0   I\01\0";
    let mut message = ResponseMessage::from(sample_message);

    let (ticks, done) = decode_historical_ticks_last(&mut message).unwrap();

    assert_eq!(ticks.len(), 7, "ticks.len()");
    assert_eq!(done, true, "done");

    assert_eq!(ticks[0].timestamp, datetime!(2023-04-10 13:30:0 UTC), "ticks[0].timestamp");
    assert_eq!(
        ticks[0].tick_attribute_last,
        TickAttributeLast {
            past_limit: false,
            unreported: false
        },
        "ticks[0].tick_attribute_last"
    );
    assert_eq!(ticks[0].price, 11.63, "ticks[0].price");
    assert_eq!(ticks[0].size, 24547, "ticks[0].size");
    assert_eq!(ticks[0].exchange, "ISLAND", "ticks[0].exchange");
    assert_eq!(ticks[0].special_conditions, " O X", "ticks[0].special_conditions");

    assert_eq!(ticks[6].timestamp, datetime!(2023-04-10 13:30:02 UTC), "ticks[6].timestamp");
    assert_eq!(
        ticks[6].tick_attribute_last,
        TickAttributeLast {
            past_limit: false,
            unreported: true
        },
        "ticks[6].tick_attribute_last"
    );
    assert_eq!(ticks[6].price, 11.73, "ticks[6].price");
    assert_eq!(ticks[6].size, 1, "ticks[6].size");
    assert_eq!(ticks[6].exchange, "FINRA", "ticks[6].exchange");
    assert_eq!(ticks[6].special_conditions, "   I", "ticks[6].special_conditions");
}

#[test]
fn test_decode_historical_tick_midpoint() {
    let sample_message = "96\09000\024\01681133398\00\091.36\00\01681133400\00\091.355\00\01681133400\00\091.35\00\01681133400\00\091.345\00\01681133400\00\091.35\00\01681133400\00\091.355\00\01681133400\00\091.35\00\01681133400\00\091.34\00\01681133400\00\091.345\00\01681133400\00\091.34\00\01681133400\00\091.345\00\01681133400\00\091.34\00\01681133400\00\091.335\00\01681133400\00\091.33\00\01681133400\00\091.325\00\01681133400\00\091.32\00\01681133400\00\091.325\00\01681133400\00\091.32\00\01681133400\00\091.315\00\01681133400\00\091.32\00\01681133400\00\091.325\00\01681133400\00\091.32\00\01681133400\00\091.315\00\01681133400\00\091.31\00\01\0";
    let mut message = ResponseMessage::from(sample_message);

    let (ticks, done) = decode_historical_ticks_mid_point(&mut message).unwrap();

    assert_eq!(ticks.len(), 24, "ticks.len()");
    assert_eq!(done, true, "done");

    assert_eq!(ticks[0].timestamp, datetime!(2023-04-10 13:29:58 UTC), "ticks[0].timestamp");
    assert_eq!(ticks[0].price, 91.36, "ticks[0].price");
    assert_eq!(ticks[0].size, 0, "ticks[0].size");

    assert_eq!(ticks[23].timestamp, datetime!(2023-04-10 13:30:00 UTC), "ticks[0].timestamp");
    assert_eq!(ticks[23].price, 91.31, "ticks[0].price");
    assert_eq!(ticks[23].size, 0, "ticks[0].size");
}
