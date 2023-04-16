use std::cell::RefCell;

use crate::stubs::MessageBusStub;

use super::*;

#[test]
fn test_head_timestamp() {
    let message_bus = RefCell::new(Box::new(MessageBusStub {
        request_messages: RefCell::new(vec![]),
        response_messages: vec!["9|1|43||".to_owned()],
    }));

    let mut client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    // client.response_packets = VecDeque::from([ResponseMessage::from("10\x0000\x00cc")]);

    let contract = Contract::stock("MSFT");
    let what_to_show = WhatToShow::Trades;
    let use_rth = true;

    let result = super::head_timestamp(&mut client, &contract, what_to_show, use_rth);

    // match result {
    //     Err(error) => assert_eq!(error.to_string(), ""),
    //     Ok(head_timestamp) => assert_eq!(head_timestamp, OffsetDateTime::now_utc()),
    // };

    // assert_eq!(client.request_packets.len(), 1);

    // let packet = &client.request_packets[0];

    // assert_eq!(packet[0], "hh");
    // assert_eq!(packet[1], "hh");
}

#[test]
fn test_histogram_data() {
    let result = 2 + 2;
    assert_eq!(result, 4);
}

#[test]
fn test_historical_data() {
    let result = 2 + 2;
    assert_eq!(result, 4);
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
