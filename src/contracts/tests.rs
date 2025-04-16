use std::sync::{Arc, RwLock};

use super::*;

use crate::stubs::MessageBusStub;

#[test]
fn request_stock_contract_details() {
    let message_bus = Arc::new(MessageBusStub{
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            "10|9001|TSLA|STK||0||SMART|USD|TSLA|NMS|NMS|76792991|0.01||ACTIVETIM,AD,ADJUST,ALERT,ALGO,ALLOC,AON,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DARKONLY,DARKPOLL,DAY,DEACT,DEACTDIS,DEACTEOD,DIS,DUR,GAT,GTC,GTD,GTT,HID,IBKRATS,ICE,IMB,IOC,LIT,LMT,LOC,MIDPX,MIT,MKT,MOC,MTL,NGCOMB,NODARK,NONALGO,OCA,OPG,OPGREROUT,PEGBENCH,PEGMID,POSTATS,POSTONLY,PREOPGRTH,PRICECHK,REL,REL2MID,RELPCTOFS,RPI,RTH,SCALE,SCALEODD,SCALERST,SIZECHK,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,SWEEP,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF|SMART,AMEX,NYSE,CBOE,PHLX,ISE,CHX,ARCA,ISLAND,DRCTEDGE,BEX,BATS,EDGEA,CSFBALGO,JEFFALGO,BYX,IEX,EDGX,FOXRIVER,PEARL,NYSENAT,LTSE,MEMX,PSX|1|0|TESLA INC|NASDAQ||Consumer, Cyclical|Auto Manufacturers|Auto-Cars/Light Trucks|US/Eastern|20221229:0400-20221229:2000;20221230:0400-20221230:2000;20221231:CLOSED;20230101:CLOSED;20230102:CLOSED;20230103:0400-20230103:2000|20221229:0930-20221229:1600;20221230:0930-20221230:1600;20221231:CLOSED;20230101:CLOSED;20230102:CLOSED;20230103:0930-20230103:1600|||1|ISIN|US88160R1014|1|||26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26||COMMON|1|1|100||".to_string(),
            "10|9001|TSLA|STK||0||AMEX|USD|TSLA|NMS|NMS|76792991|0.01||ACTIVETIM,AD,ADJUST,ALERT,ALLOC,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DAY,DEACT,DEACTDIS,DEACTEOD,GAT,GTC,GTD,GTT,HID,IOC,LIT,LMT,MIT,MKT,MTL,NGCOMB,NONALGO,OCA,PEGBENCH,SCALE,SCALERST,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF|SMART,AMEX,NYSE,CBOE,PHLX,ISE,CHX,ARCA,ISLAND,DRCTEDGE,BEX,BATS,EDGEA,CSFBALGO,JEFFALGO,BYX,IEX,EDGX,FOXRIVER,PEARL,NYSENAT,LTSE,MEMX,PSX|1|0|TESLA INC|NASDAQ||Consumer, Cyclical|Auto Manufacturers|Auto-Cars/Light Trucks|US/Eastern|20221229:0700-20221229:2000;20221230:0700-20221230:2000;20221231:CLOSED;20230101:CLOSED;20230102:CLOSED;20230103:0700-20230103:2000|20221229:0700-20221229:2000;20221230:0700-20221230:2000;20221231:CLOSED;20230101:CLOSED;20230102:CLOSED;20230103:0700-20230103:2000|||1|ISIN|US88160R1014|1|||26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26||COMMON|1|1|100||".to_string(),
            "52|1|9001||".to_string(),
        ]
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let contract = Contract::stock("TSLA");

    let results = client.contract_details(&contract);

    let request_messages = client.message_bus.request_messages();

    assert_eq!(request_messages[0].encode_simple(), "9|8|9000|0|TSLA|STK||0|||SMART||USD|||0|||");

    assert!(results.is_ok(), "failed to encode request: {:?}", results.err());

    let contracts: Vec<ContractDetails> = results.unwrap();
    assert_eq!(2, contracts.len());

    assert_eq!(contracts[0].contract.exchange, "SMART");
    assert_eq!(contracts[1].contract.exchange, "AMEX");

    assert_eq!(contracts[0].contract.symbol, "TSLA");
    assert_eq!(contracts[0].contract.security_type, SecurityType::Stock);
    assert_eq!(contracts[0].contract.currency, "USD");
    assert_eq!(contracts[0].contract.contract_id, 76792991);
    assert_eq!(
        contracts[0].order_types,
        vec![
            "ACTIVETIM",
            "AD",
            "ADJUST",
            "ALERT",
            "ALGO",
            "ALLOC",
            "AON",
            "AVGCOST",
            "BASKET",
            "BENCHPX",
            "CASHQTY",
            "COND",
            "CONDORDER",
            "DARKONLY",
            "DARKPOLL",
            "DAY",
            "DEACT",
            "DEACTDIS",
            "DEACTEOD",
            "DIS",
            "DUR",
            "GAT",
            "GTC",
            "GTD",
            "GTT",
            "HID",
            "IBKRATS",
            "ICE",
            "IMB",
            "IOC",
            "LIT",
            "LMT",
            "LOC",
            "MIDPX",
            "MIT",
            "MKT",
            "MOC",
            "MTL",
            "NGCOMB",
            "NODARK",
            "NONALGO",
            "OCA",
            "OPG",
            "OPGREROUT",
            "PEGBENCH",
            "PEGMID",
            "POSTATS",
            "POSTONLY",
            "PREOPGRTH",
            "PRICECHK",
            "REL",
            "REL2MID",
            "RELPCTOFS",
            "RPI",
            "RTH",
            "SCALE",
            "SCALEODD",
            "SCALERST",
            "SIZECHK",
            "SNAPMID",
            "SNAPMKT",
            "SNAPREL",
            "STP",
            "STPLMT",
            "SWEEP",
            "TRAIL",
            "TRAILLIT",
            "TRAILLMT",
            "TRAILMIT",
            "WHATIF"
        ]
    );
    assert_eq!(
        contracts[0].valid_exchanges,
        vec![
            "SMART", "AMEX", "NYSE", "CBOE", "PHLX", "ISE", "CHX", "ARCA", "ISLAND", "DRCTEDGE", "BEX", "BATS", "EDGEA", "CSFBALGO", "JEFFALGO",
            "BYX", "IEX", "EDGX", "FOXRIVER", "PEARL", "NYSENAT", "LTSE", "MEMX", "PSX"
        ]
    );
    assert_eq!(contracts[0].price_magnifier, 1);
    assert_eq!(contracts[0].under_contract_id, 0);
    assert_eq!(contracts[0].long_name, "TESLA INC");
    assert_eq!(contracts[0].contract.primary_exchange, "NASDAQ");
    assert_eq!(contracts[0].contract_month, "");
    assert_eq!(contracts[0].industry, "Consumer, Cyclical");
    assert_eq!(contracts[0].category, "Auto Manufacturers");
    assert_eq!(contracts[0].subcategory, "Auto-Cars/Light Trucks");
    assert_eq!(contracts[0].time_zone_id, "US/Eastern");
    assert_eq!(
        contracts[0].trading_hours,
        vec![
            "20221229:0400-20221229:2000",
            "20221230:0400-20221230:2000",
            "20221231:CLOSED",
            "20230101:CLOSED",
            "20230102:CLOSED",
            "20230103:0400-20230103:2000"
        ]
    );
    assert_eq!(
        contracts[0].liquid_hours,
        vec![
            "20221229:0930-20221229:1600",
            "20221230:0930-20221230:1600",
            "20221231:CLOSED",
            "20230101:CLOSED",
            "20230102:CLOSED",
            "20230103:0930-20230103:1600"
        ]
    );
    assert_eq!(contracts[0].ev_rule, "");
    assert_eq!(contracts[0].ev_multiplier, 0.0);
    assert_eq!(contracts[0].sec_id_list.len(), 1);
    assert_eq!(contracts[0].sec_id_list[0].tag, "ISIN");
    assert_eq!(contracts[0].sec_id_list[0].value, "US88160R1014");
    assert_eq!(contracts[0].agg_group, 1);
    assert_eq!(
        contracts[0].market_rule_ids,
        vec![
            "26", "26", "26", "26", "26", "26", "26", "26", "26", "26", "26", "26", "26", "26", "26", "26", "26", "26", "26", "26", "26", "26", "26",
            "26"
        ]
    );
    assert_eq!(contracts[0].stock_type, "COMMON");
    assert_eq!(contracts[0].min_size, 1.0);
    assert_eq!(contracts[0].size_increment, 1.0);
    assert_eq!(contracts[0].suggested_size_increment, 100.0);
}

#[test]
fn request_bond_contract_details() {}

#[test]
fn request_future_contract_details() {}

#[test]
fn test_read_last_trade_date() {
    // let mut contract = ContractDetails::default();

    // handles blank string
    // let result = read_last_trade_date(&mut contract, "", false);
    // assert!(!result.is_err(), "unexpected error {:?}", result);

    // handles non bond contracts

    // handles bond contracts
}

#[test]
fn request_matching_symbols() {}

#[test]
fn test_contract_option_builder() {
    let contract = Contract::option("AAPL", "20231215", 150.0, "C");
    assert_eq!(contract.symbol, "AAPL");
    assert_eq!(contract.last_trade_date_or_contract_month, "20231215");
    assert_eq!(contract.strike, 150.0);
    assert_eq!(contract.right, "C");
    assert_eq!(contract.currency, "USD");
    assert_eq!(contract.exchange, "SMART");
    assert_eq!(contract.security_type, SecurityType::Option);
}

#[test]
fn test_contract_futures_builder() {
    let contract = Contract::futures("ES");
    assert_eq!(contract.symbol, "ES");
    assert_eq!(contract.currency, "USD");
    assert_eq!(contract.security_type, SecurityType::Future);
}

#[test]
fn test_contract_crypto_builder() {
    let contract = Contract::crypto("BTC");
    assert_eq!(contract.symbol, "BTC");
    assert_eq!(contract.security_type, SecurityType::Crypto);
    assert_eq!(contract.exchange, "PAXOS");
    assert_eq!(contract.currency, "USD");
}

#[test]
fn test_security_type_from_strings() {
    let cases = vec![
        ("STK", SecurityType::Stock),
        ("OPT", SecurityType::Option),
        ("FUT", SecurityType::Future),
        ("IND", SecurityType::Index),
        ("FOP", SecurityType::FuturesOption),
        ("CASH", SecurityType::ForexPair),
        ("BAG", SecurityType::Spread),
        ("WAR", SecurityType::Warrant),
        ("BOND", SecurityType::Bond),
        ("CMDTY", SecurityType::Commodity),
        ("NEWS", SecurityType::News),
        ("FUND", SecurityType::MutualFund),
        ("CRYPTO", SecurityType::Crypto),
    ];

    for (input, expected) in cases {
        assert_eq!(SecurityType::from(input), expected);
    }
}

#[test]
fn test_tag_value_to_field() {
    let tags = vec![
        TagValue {
            tag: "foo".into(),
            value: "bar".into(),
        },
        TagValue {
            tag: "baz".into(),
            value: "qux".into(),
        },
    ];

    assert_eq!(tags.to_field(), "foo=bar;baz=qux;");
}

#[test]
fn test_contract_push_fields_order() {
    let contract = Contract::option("AAPL", "20231215", 150.0, "C");
    let mut message = RequestMessage::default();
    contract.push_fields(&mut message);

    let expected = "0\0AAPL\0OPT\020231215\0150\0C\0\0SMART\0\0USD\0\0\00\0";

    assert_eq!(message.encode(), expected);
}
