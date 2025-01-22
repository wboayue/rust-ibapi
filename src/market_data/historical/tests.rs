use std::sync::{Arc, RwLock};

use time::macros::datetime;

use crate::market_data::historical::ToDuration;
use crate::messages::OutgoingMessages;
use crate::stubs::MessageBusStub;

use super::*;

#[test]
fn test_head_timestamp() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec!["88|9000|1678323335|".to_owned()],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let contract = Contract::stock("MSFT");
    let what_to_show = WhatToShow::Trades;
    let use_rth = true;

    let head_timestamp = client
        .head_timestamp(&contract, what_to_show, use_rth)
        .expect("head timestamp request failed");

    assert_eq!(head_timestamp, OffsetDateTime::from_unix_timestamp(1678323335).unwrap(), "bar.date");

    let request_messages = client.message_bus.request_messages();

    let head_timestamp_request = &request_messages[0];
    assert_eq!(
        head_timestamp_request[0],
        OutgoingMessages::RequestHeadTimestamp.to_field(),
        "message.message_type"
    );
    assert_eq!(head_timestamp_request[1], "9000", "message.request_id");
    assert_eq!(head_timestamp_request[2], contract.contract_id.to_field(), "message.contract_id");
    assert_eq!(head_timestamp_request[3], contract.symbol.to_field(), "message.symbol");
    assert_eq!(head_timestamp_request[4], contract.security_type.to_field(), "message.security_type");
    assert_eq!(
        head_timestamp_request[5],
        contract.last_trade_date_or_contract_month.to_field(),
        "message.last_trade_date_or_contract_month"
    );
    assert_eq!(head_timestamp_request[6], contract.strike.to_field(), "message.strike");
    assert_eq!(head_timestamp_request[7], contract.right.to_field(), "message.right");
    assert_eq!(head_timestamp_request[8], contract.multiplier.to_field(), "message.multiplier");
    assert_eq!(head_timestamp_request[9], contract.exchange.to_field(), "message.exchange");
    assert_eq!(
        head_timestamp_request[10],
        contract.primary_exchange.to_field(),
        "message.primary_exchange"
    );
    assert_eq!(head_timestamp_request[11], contract.currency.to_field(), "message.currency");
    assert_eq!(head_timestamp_request[12], contract.local_symbol.to_field(), "message.local_symbol");
    assert_eq!(head_timestamp_request[13], contract.trading_class.to_field(), "message.trading_class");
    assert_eq!(head_timestamp_request[14], contract.include_expired.to_field(), "message.include_expired");
    assert_eq!(head_timestamp_request[15], use_rth.to_field(), "message.use_rth");
    assert_eq!(head_timestamp_request[16], what_to_show.to_field(), "message.what_to_show");
    assert_eq!(head_timestamp_request[17], "2", "message.date_format");
}

#[test]
fn test_histogram_data() {
    let result = 2 + 2;
    assert_eq!(result, 4);
}

#[test]
fn test_historical_data() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            "17\09000\020230413  16:31:22\020230415  16:31:22\02\020230413\0182.9400\0186.5000\0180.9400\0185.9000\0948837.22\0184.869\0324891\020230414\0183.8800\0186.2800\0182.0100\0185.0000\0810998.27\0183.9865\0277547\0".to_owned()
        ],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let contract = Contract::stock("MSFT");
    let interval_end = datetime!(2023-04-15 16:31:22 UTC);
    let duration = 2.days();
    let bar_size = BarSize::Hour;
    let what_to_show = WhatToShow::Trades;
    let use_rth = true;

    let historical_data = client
        .historical_data(&contract, Some(interval_end), duration, bar_size, what_to_show, use_rth)
        .expect("historical data request failed");

    // Assert Response

    assert_eq!(historical_data.start, datetime!(2023-04-13 16:31:22 UTC), "historical_data.start");
    assert_eq!(historical_data.end, datetime!(2023-04-15 16:31:22 UTC), "historical_data.end");
    assert_eq!(historical_data.bars.len(), 2, "historical_data.bars.len()");

    assert_eq!(historical_data.bars[0].date, datetime!(2023-04-13 00:00:00 UTC), "bar.date");
    assert_eq!(historical_data.bars[0].open, 182.94, "bar.open");
    assert_eq!(historical_data.bars[0].high, 186.50, "bar.high");
    assert_eq!(historical_data.bars[0].low, 180.94, "bar.low");
    assert_eq!(historical_data.bars[0].close, 185.90, "bar.close");
    assert_eq!(historical_data.bars[0].volume, 948837.22, "bar.volume");
    assert_eq!(historical_data.bars[0].wap, 184.869, "bar.wap");
    assert_eq!(historical_data.bars[0].count, 324891, "bar.count");

    // Assert Request

    let request_messages = client.message_bus.request_messages();

    let head_timestamp_request = &request_messages[0];
    assert_eq!(
        head_timestamp_request[0],
        OutgoingMessages::RequestHistoricalData.to_field(),
        "message.message_type"
    );
    assert_eq!(head_timestamp_request[1], "9000", "message.request_id");
    assert_eq!(head_timestamp_request[2], contract.contract_id.to_field(), "message.contract_id");
    assert_eq!(head_timestamp_request[3], contract.symbol.to_field(), "message.symbol");
    assert_eq!(head_timestamp_request[4], contract.security_type.to_field(), "message.security_type");
    assert_eq!(
        head_timestamp_request[5],
        contract.last_trade_date_or_contract_month.to_field(),
        "message.last_trade_date_or_contract_month"
    );
    assert_eq!(head_timestamp_request[6], contract.strike.to_field(), "message.strike");
    assert_eq!(head_timestamp_request[7], contract.right.to_field(), "message.right");
    assert_eq!(head_timestamp_request[8], contract.multiplier.to_field(), "message.multiplier");
    assert_eq!(head_timestamp_request[9], contract.exchange.to_field(), "message.exchange");
    assert_eq!(
        head_timestamp_request[10],
        contract.primary_exchange.to_field(),
        "message.primary_exchange"
    );
    assert_eq!(head_timestamp_request[11], contract.currency.to_field(), "message.currency");
    assert_eq!(head_timestamp_request[12], contract.local_symbol.to_field(), "message.local_symbol");
    assert_eq!(head_timestamp_request[13], contract.trading_class.to_field(), "message.trading_class");
    assert_eq!(head_timestamp_request[14], contract.include_expired.to_field(), "message.include_expired");
    assert_eq!(head_timestamp_request[15], interval_end.to_field(), "message.interval_end");
    assert_eq!(head_timestamp_request[16], bar_size.to_field(), "message.bar_size");
    assert_eq!(head_timestamp_request[17], duration.to_field(), "message.duration");
    assert_eq!(head_timestamp_request[18], use_rth.to_field(), "message.use_rth");
    assert_eq!(head_timestamp_request[19], what_to_show.to_field(), "message.what_to_show");
    assert_eq!(head_timestamp_request[20], "2", "message.date_format");
    assert_eq!(head_timestamp_request[21], "0", "message.keep_up_to_data");
    assert_eq!(head_timestamp_request[22], "", "message.chart_options");
}

#[test]
fn test_bar_size() {
    assert_eq!(BarSize::Sec.to_string(), "1 sec");
    assert_eq!(BarSize::Sec5.to_string(), "5 secs");
    assert_eq!(BarSize::Sec15.to_string(), "15 secs");
    assert_eq!(BarSize::Sec30.to_string(), "30 secs");
    assert_eq!(BarSize::Min.to_string(), "1 min");
    assert_eq!(BarSize::Min2.to_string(), "2 mins");
    assert_eq!(BarSize::Min3.to_string(), "3 mins");
    assert_eq!(BarSize::Min5.to_string(), "5 mins");
    assert_eq!(BarSize::Min15.to_string(), "15 mins");
    assert_eq!(BarSize::Min30.to_string(), "30 mins");
    assert_eq!(BarSize::Hour.to_string(), "1 hour");
    assert_eq!(BarSize::Day.to_string(), "1 day");
}

#[test]
fn test_what_to_show() {
    assert_eq!(WhatToShow::Trades.to_string(), "TRADES");
    assert_eq!(WhatToShow::MidPoint.to_string(), "MIDPOINT");
    assert_eq!(WhatToShow::Bid.to_string(), "BID");
    assert_eq!(WhatToShow::Ask.to_string(), "ASK");
    assert_eq!(WhatToShow::BidAsk.to_string(), "BID_ASK");
    assert_eq!(WhatToShow::HistoricalVolatility.to_string(), "HISTORICAL_VOLATILITY");
    assert_eq!(WhatToShow::OptionImpliedVolatility.to_string(), "OPTION_IMPLIED_VOLATILITY");
    assert_eq!(WhatToShow::FeeRate.to_string(), "FEE_RATE");
    assert_eq!(WhatToShow::Schedule.to_string(), "SCHEDULE");
    assert_eq!(WhatToShow::AdjustedLast.to_string(), "ADJUSTED_LAST");
}

#[test]
fn test_duration() {
    assert_eq!(Duration::SECOND.to_field(), "1 S");
    assert_eq!(Duration::DAY.to_field(), "1 D");
    assert_eq!(Duration::WEEK.to_field(), "1 W");
    assert_eq!(Duration::MONTH.to_field(), "1 M");
    assert_eq!(Duration::YEAR.to_field(), "1 Y");

    assert_eq!(2.seconds().to_field(), "2 S");
    assert_eq!(3.days().to_field(), "3 D");
    assert_eq!(4.weeks().to_field(), "4 W");
    assert_eq!(5.months().to_field(), "5 M");
    assert_eq!(6.years().to_field(), "6 Y");
}
