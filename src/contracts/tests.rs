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
#[ignore = "reason: need sample messages"]
fn request_bond_contract_details() {
    let message_bus = Arc::new(MessageBusStub{
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Format similar to request_stock_contract_details but with bond-specific fields
            "10|9001|TLT|BOND|20420815|0||||USD|TLT|US Treasury Bond|BOND|12345|0.01|1000|SMART|NYSE|SMART|NYSE|1|0|US Treasury Bond|SMART||Government||US/Eastern|20221229:0400-20221229:2000;20221230:0400-20221230:2000|20221229:0930-20221229:1600;20221230:0930-20221230:1600|||1|CUSIP|912810TL8|1|||26|20420815|GOVT|1|1|2.25|0|20420815|20120815|20320815|CALL|100.0|1|Government Bond Notes|0.1|0.01|1|".to_string(),
            "52|1|9001||".to_string(),
        ]
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    // Create a bond contract
    let mut contract = Contract::default();
    contract.symbol = "TLT".to_string();
    contract.security_type = SecurityType::Bond;
    contract.exchange = "SMART".to_string();
    contract.currency = "USD".to_string();

    let results = client.contract_details(&contract);

    let request_messages = client.message_bus.request_messages();

    // Check if the request was encoded correctly
    assert_eq!(request_messages[0].encode_simple(), "9|8|9000|0|TLT|BOND||0|||SMART||USD|||0|||");

    assert!(results.is_ok(), "failed to encode request: {:?}", results.err());

    let contracts: Vec<ContractDetails> = results.unwrap();
    assert_eq!(1, contracts.len());

    // Check basic contract fields
    assert_eq!(contracts[0].contract.symbol, "TLT");
    assert_eq!(contracts[0].contract.security_type, SecurityType::Bond);
    assert_eq!(contracts[0].contract.currency, "USD");
    assert_eq!(contracts[0].contract.contract_id, 12345);

    // Check bond-specific fields
    assert_eq!(contracts[0].contract.last_trade_date_or_contract_month, "20420815");
    assert_eq!(contracts[0].cusip, "912810TL8");
    assert_eq!(contracts[0].coupon, 2.25);
    assert_eq!(contracts[0].maturity, "20420815");
    assert_eq!(contracts[0].issue_date, "20120815");
    assert_eq!(contracts[0].next_option_date, "20320815");
    assert_eq!(contracts[0].next_option_type, "CALL");
    assert_eq!(contracts[0].next_option_partial, true);
    assert_eq!(contracts[0].notes, "Government Bond Notes");
}

#[test]
fn request_future_contract_details() {
    let message_bus = Arc::new(MessageBusStub{
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            "10|9000|ES|FUT|20250620 08:30 US/Central|0||CME|USD|ESM5|ES|ES|620731015|0.25|50|ACTIVETIM,AD,ADJUST,ALERT,ALGO,ALLOC,AVGCOST,BASKET,BENCHPX,COND,CONDORDER,DAY,DEACT,DEACTDIS,DEACTEOD,GAT,GTC,GTD,GTT,HID,ICE,IOC,LIT,LMT,LTH,MIT,MKT,MTL,NGCOMB,NONALGO,OCA,PEGBENCH,SCALE,SCALERST,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF|CME,QBALGO|1|11004968|E-mini S&P 500||202506||||US/Central|20250521:1700-20250522:1600;20250522:1700-20250523:1600;20250524:CLOSED;20250525:1700-20250526:1200;20250526:1700-20250527:1600;20250527:1700-20250528:1600|20250522:0830-20250522:1600;20250523:0830-20250523:1600;20250524:CLOSED;20250525:1700-20250526:1200;20250527:0830-20250527:1600;20250527:1700-20250528:1600|||0|2147483647|ES|IND|67,67|20250620||1|1|1|".to_string(),
            "52|1|9000|".to_string(),
        ]
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    // Create a future contract
    let mut contract = Contract::default();
    contract.symbol = "ES".to_string();
    contract.security_type = SecurityType::Future;
    contract.last_trade_date_or_contract_month = "202506".to_string();
    contract.exchange = "GLOBEX".to_string();
    contract.currency = "USD".to_string();

    let results = client.contract_details(&contract);

    let request_messages = client.message_bus.request_messages();

    // Check if the request was encoded correctly
    assert_eq!(request_messages[0].encode_simple(), "9|8|9000|0|ES|FUT|202506|0|||GLOBEX||USD|||0|||");

    assert!(results.is_ok(), "failed to encode request: {:?}", results.err());

    let contracts: Vec<ContractDetails> = results.unwrap();
    assert_eq!(1, contracts.len());

    // Check basic contract fields
    assert_eq!(contracts[0].contract.symbol, "ES");
    assert_eq!(contracts[0].contract.security_type, SecurityType::Future);
    assert_eq!(contracts[0].contract.currency, "USD");
    assert_eq!(contracts[0].contract.contract_id, 620731015);

    // Check future-specific fields
    assert_eq!(contracts[0].contract.last_trade_date_or_contract_month, "20250620");
    assert_eq!(contracts[0].contract.multiplier, "50");
    assert_eq!(contracts[0].contract.local_symbol, "ESM5");
    assert_eq!(contracts[0].contract.trading_class, "ES");
    assert_eq!(contracts[0].contract.exchange, "CME");
    assert_eq!(contracts[0].min_tick, 0.25);
    assert_eq!(contracts[0].market_name, "ES");
    assert_eq!(contracts[0].contract_month, "202506");
    assert_eq!(contracts[0].real_expiration_date, "20250620");
}

#[test]
fn test_contract_constructors() {
    // Test stock constructor
    let stock = Contract::stock("AAPL");
    assert_eq!(stock.symbol, "AAPL", "stock.symbol");
    assert_eq!(stock.security_type, SecurityType::Stock, "stock.security_type");
    assert_eq!(stock.currency, "USD", "stock.currency");
    assert_eq!(stock.exchange, "SMART", "stock.exchange");

    // Test futures constructor
    let futures = Contract::futures("ES");
    assert_eq!(futures.symbol, "ES", "futures.symbol");
    assert_eq!(futures.security_type, SecurityType::Future, "futures.security_type");
    assert_eq!(futures.currency, "USD", "futures.currency");
    assert_eq!(futures.exchange, "", "futures.exchange");

    // Test crypto constructor
    let crypto = Contract::crypto("BTC");
    assert_eq!(crypto.symbol, "BTC", "crypto.symbol");
    assert_eq!(crypto.security_type, SecurityType::Crypto, "crypto.security_type");
    assert_eq!(crypto.currency, "USD", "crypto.currency");
    assert_eq!(crypto.exchange, "PAXOS", "crypto.exchange");

    // Test news constructor
    let news = Contract::news("BZ");
    assert_eq!(news.symbol, "BZ:BZ_ALL", "news.symbol");
    assert_eq!(news.security_type, SecurityType::News, "news.security_type");
    assert_eq!(news.exchange, "BZ", "news.exchange");

    // Test option constructor
    let option = Contract::option("AAPL", "20231215", 150.0, "C");
    assert_eq!(option.symbol, "AAPL", "option.symbol");
    assert_eq!(option.security_type, SecurityType::Option, "option.security_type");
    assert_eq!(
        option.last_trade_date_or_contract_month, "20231215",
        "option.last_trade_date_or_contract_month"
    );
    assert_eq!(option.strike, 150.0, "option.strike");
    assert_eq!(option.right, "C", "option.right");
    assert_eq!(option.exchange, "SMART", "option.exchange");
    assert_eq!(option.currency, "USD", "option.currency");
}

#[test]
fn test_security_type_from() {
    // Test all known security types
    assert_eq!(SecurityType::from("STK"), SecurityType::Stock, "STK should be Stock");
    assert_eq!(SecurityType::from("OPT"), SecurityType::Option, "OPT should be Option");
    assert_eq!(SecurityType::from("FUT"), SecurityType::Future, "FUT should be Future");
    assert_eq!(SecurityType::from("IND"), SecurityType::Index, "IND should be Index");
    assert_eq!(SecurityType::from("FOP"), SecurityType::FuturesOption, "FOP should be FuturesOption");
    assert_eq!(SecurityType::from("CASH"), SecurityType::ForexPair, "CASH should be ForexPair");
    assert_eq!(SecurityType::from("BAG"), SecurityType::Spread, "BAG should be Spread");
    assert_eq!(SecurityType::from("WAR"), SecurityType::Warrant, "WAR should be Warrant");
    assert_eq!(SecurityType::from("BOND"), SecurityType::Bond, "BOND should be Bond");
    assert_eq!(SecurityType::from("CMDTY"), SecurityType::Commodity, "CMDTY should be Commodity");
    assert_eq!(SecurityType::from("NEWS"), SecurityType::News, "NEWS should be News");
    assert_eq!(SecurityType::from("FUND"), SecurityType::MutualFund, "FUND should be MutualFund");
    assert_eq!(SecurityType::from("CRYPTO"), SecurityType::Crypto, "CRYPTO should be Crypto");
    assert_eq!(SecurityType::from("CFD"), SecurityType::CFD, "CFD should be CFD");

    // Test unknown security type
    match SecurityType::from("UNKNOWN") {
        SecurityType::Other(name) => assert_eq!(name, "UNKNOWN", "Other should contain original string"),
        _ => panic!("Expected SecurityType::Other for unknown type"),
    }
}

#[test]
fn request_matching_symbols() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            "79|9000|16|76792991|TSLA|STK|ISLAND|USD|5|CFD|OPT|IOPT|WAR|BAG|78046366|TL0|STK|IBIS|EUR|2|CFD|IOPT|660309051|TSLT|STK|BATS|USD|2|OPT|BAG|684954177|TSL3|STK|LSEETF|USD|0|144303597|TSLA|STK|MEXI|MXN|0|603849333|YTSL|STK|AEQLIT|CAD|0|660309044|TSLZ|STK|BATS|USD|2|OPT|BAG|754543238|TSLY|STK|TSE|CAD|0|681568926|3TSE|STK|LSEETF|EUR|0|684954172|3TSL|STK|LSEETF|GBP|0|425172013|2TSL|STK|LSEETF|GBP|0|74619514|TXL|STK|VALUE|CAD|0|172604402|TSLA|STK|EBS|CHF|0|272100479|TES1|STK|BM|EUR|0|425172008|TSL2|STK|LSEETF|USD|0|-1||BOND|||0|".to_string(),
        ],
    });

    let tt = decoders::decode_contract_descriptions(
        server_versions::HMDS_MARKET_DATA_IN_SHARES,
        &mut ResponseMessage::from_simple(&message_bus.response_messages[0]),
    );
    assert!(tt.is_ok(), "failed to decode response: {:?}", tt.err());

    let client = Client::stubbed(message_bus, server_versions::REQ_MATCHING_SYMBOLS);

    let pattern = "TSLA";
    let results = client.matching_symbols(pattern);

    let request_messages = client.message_bus.request_messages();

    // Check if the request was encoded correctly
    assert_eq!(request_messages[0].encode_simple(), "81|9000|TSLA|");

    assert!(results.is_ok(), "failed to send request: {:?}", results.err());

    // Collect the iterator into a vector to test each item
    let contract_descriptions: Vec<ContractDescription> = results.unwrap().collect();
    assert_eq!(16, contract_descriptions.len());

    // Check first contract description
    assert_eq!(contract_descriptions[0].contract.contract_id, 76792991);
    assert_eq!(contract_descriptions[0].contract.symbol, "TSLA");
    assert_eq!(contract_descriptions[0].contract.security_type, SecurityType::Stock);
    assert_eq!(contract_descriptions[0].contract.primary_exchange, "ISLAND");
    assert_eq!(contract_descriptions[0].contract.currency, "USD");
    assert_eq!(contract_descriptions[0].derivative_security_types.len(), 5);
    assert_eq!(contract_descriptions[0].derivative_security_types[0], "CFD");
    assert_eq!(contract_descriptions[0].derivative_security_types[1], "OPT");

    // Check second contract description
    assert_eq!(contract_descriptions[1].contract.contract_id, 78046366);
    assert_eq!(contract_descriptions[1].contract.symbol, "TL0");
    assert_eq!(contract_descriptions[1].contract.security_type, SecurityType::Stock);
    assert_eq!(contract_descriptions[1].contract.primary_exchange, "IBIS");
    assert_eq!(contract_descriptions[1].contract.currency, "EUR");
    assert_eq!(contract_descriptions[1].derivative_security_types.len(), 2);
    assert_eq!(contract_descriptions[1].derivative_security_types[0], "CFD");
    assert_eq!(contract_descriptions[1].derivative_security_types[1], "IOPT");
}

#[test]
fn test_verify_contract() {
    // Test for security_id_type and security_id validation
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![],
    });

    // Test with old server version (should fail)
    let client = Client::stubbed(message_bus.clone(), server_versions::SEC_ID_TYPE - 1);
    let contract = Contract {
        security_id_type: "ISIN".to_string(),
        security_id: "US0378331005".to_string(),
        ..Default::default()
    };
    let result = verify_contract(&client, &contract);
    assert!(result.is_err(), "Should fail with old server version for security_id_type");

    // Test for trading_class validation
    let client = Client::stubbed(message_bus.clone(), server_versions::TRADING_CLASS - 1);
    let contract = Contract {
        trading_class: "AAPL".to_string(),
        ..Default::default()
    };
    let result = verify_contract(&client, &contract);
    assert!(result.is_err(), "Should fail with old server version for trading_class");

    // Test for primary_exchange validation
    let client = Client::stubbed(message_bus.clone(), server_versions::LINKING - 1);
    let contract = Contract {
        primary_exchange: "NASDAQ".to_string(),
        ..Default::default()
    };
    let result = verify_contract(&client, &contract);
    assert!(result.is_err(), "Should fail with old server version for primary_exchange");

    // Test for issuer_id validation
    let client = Client::stubbed(message_bus.clone(), server_versions::BOND_ISSUERID - 1);
    let contract = Contract {
        issuer_id: "ISSUER123".to_string(),
        ..Default::default()
    };
    let result = verify_contract(&client, &contract);
    assert!(result.is_err(), "Should fail with old server version for issuer_id");

    // Test with newest server version (all should pass)
    let client = Client::stubbed(message_bus, server_versions::BOND_ISSUERID + 1);
    let contract = Contract {
        security_id_type: "ISIN".to_string(),
        security_id: "US0378331005".to_string(),
        trading_class: "AAPL".to_string(),
        primary_exchange: "NASDAQ".to_string(),
        issuer_id: "ISSUER123".to_string(),
        ..Default::default()
    };
    let result = verify_contract(&client, &contract);
    assert!(result.is_ok(), "Should succeed with newest server version");
}

#[test]
fn test_combo_leg_open_close() {
    // Test From<i32> implementation
    assert_eq!(ComboLegOpenClose::from(0), ComboLegOpenClose::Same, "0 should be Same");
    assert_eq!(ComboLegOpenClose::from(1), ComboLegOpenClose::Open, "1 should be Open");
    assert_eq!(ComboLegOpenClose::from(2), ComboLegOpenClose::Close, "2 should be Close");
    assert_eq!(ComboLegOpenClose::from(3), ComboLegOpenClose::Unknown, "3 should be Unknown");

    // Test ToField implementation
    assert_eq!(ComboLegOpenClose::Same.to_field(), "0", "Same should be 0");
    assert_eq!(ComboLegOpenClose::Open.to_field(), "1", "Open should be 1");
    assert_eq!(ComboLegOpenClose::Close.to_field(), "2", "Close should be 2");
    assert_eq!(ComboLegOpenClose::Unknown.to_field(), "3", "Unknown should be 3");

    // Test Default implementation
    assert_eq!(ComboLegOpenClose::default(), ComboLegOpenClose::Same, "Default should be Same");
}

#[test]
#[should_panic(expected = "unsupported value")]
fn test_combo_leg_open_close_panic() {
    // Test panic with invalid value
    let _invalid = ComboLegOpenClose::from(4); // This should panic
}

#[test]
fn test_tag_value_to_field() {
    // Test with multiple TagValue items
    let tag_values = vec![
        TagValue {
            tag: "TAG1".to_string(),
            value: "VALUE1".to_string(),
        },
        TagValue {
            tag: "TAG2".to_string(),
            value: "VALUE2".to_string(),
        },
        TagValue {
            tag: "TAG3".to_string(),
            value: "VALUE3".to_string(),
        },
    ];

    assert_eq!(
        tag_values.to_field(),
        "TAG1=VALUE1;TAG2=VALUE2;TAG3=VALUE3;",
        "Tag values should be formatted as TAG=VALUE; pairs"
    );

    // Test with a single TagValue
    let single_tag_value = vec![TagValue {
        tag: "SINGLE_TAG".to_string(),
        value: "SINGLE_VALUE".to_string(),
    }];

    assert_eq!(
        single_tag_value.to_field(),
        "SINGLE_TAG=SINGLE_VALUE;",
        "Single tag value should be formatted as TAG=VALUE;"
    );

    // Test with empty vec
    let empty: Vec<TagValue> = vec![];
    assert_eq!(empty.to_field(), "", "Empty vec should result in empty string");

    // Test with empty tag/value
    let empty_fields = vec![TagValue {
        tag: "".to_string(),
        value: "".to_string(),
    }];

    assert_eq!(empty_fields.to_field(), "=;", "Empty tag/value should be formatted as =;");
}

#[test]
fn test_market_rule() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Market rule format: message_type, market_rule_id, price_increment_count, low_edge, increment, low_edge, increment, ...
            "93|26|3|0|0.01|100|0.05|1000|0.1|".to_string(),
        ],
    });

    let client = Client::stubbed(message_bus, server_versions::MARKET_RULES);

    // Call the market_rule function with a specific rule ID
    let market_rule_id = 26;
    let result = client.market_rule(market_rule_id);

    // Check request encoding
    let request_messages = client.message_bus.request_messages();
    assert_eq!(request_messages[0].encode_simple(), "91|26|");

    // Verify result
    assert!(result.is_ok(), "failed to get market rule: {:?}", result.err());

    let market_rule = result.unwrap();

    // Verify market rule details
    assert_eq!(market_rule.market_rule_id, 26, "market_rule.market_rule_id");
    assert_eq!(market_rule.price_increments.len(), 3, "market_rule.price_increments.len()");

    // Check first price increment
    assert_eq!(market_rule.price_increments[0].low_edge, 0.0, "price_increments[0].low_edge");
    assert_eq!(market_rule.price_increments[0].increment, 0.01, "price_increments[0].increment");

    // Check second price increment
    assert_eq!(market_rule.price_increments[1].low_edge, 100.0, "price_increments[1].low_edge");
    assert_eq!(market_rule.price_increments[1].increment, 0.05, "price_increments[1].increment");

    // Check third price increment
    assert_eq!(market_rule.price_increments[2].low_edge, 1000.0, "price_increments[2].low_edge");
    assert_eq!(market_rule.price_increments[2].increment, 0.1, "price_increments[2].increment");

    // Test error case with server version too old
    let old_client = Client::stubbed(
        Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        }),
        server_versions::MARKET_RULES - 1,
    );

    let error_result = old_client.market_rule(market_rule_id);
    assert!(error_result.is_err(), "Should fail with old server version");
}

#[test]
fn test_calculate_option_price() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Option computation format: message_type, request_id, tick_type, tick_attribute, implied_vol, delta, option_price, pv_dividend, gamma, vega, theta, underlying_price
            "21|9000|13|1|0.3|0.65|5.75|0.5|0.05|0.15|0.01|145.0|".to_string(),
        ],
    });

    let client = Client::stubbed(message_bus, server_versions::REQ_CALC_OPTION_PRICE);

    // Create test option contract
    let contract = Contract::option("AAPL", "20231215", 150.0, "C");

    // Test input parameters
    let volatility = 0.3;
    let underlying_price = 145.0;

    // Call the calculate_option_price function
    let result = client.calculate_option_price(&contract, volatility, underlying_price);

    // Check request encoding
    let request_messages = client.message_bus.request_messages();

    // Request format: message_type, version, request_id, contract fields..., volatility, underlying_price, empty
    assert!(
        request_messages[0].encode_simple().contains("0|AAPL|OPT|20231215|150|C||SMART"),
        "Unexpected request message format"
    );

    // Verify result
    assert!(result.is_ok(), "failed to calculate option price: {:?}", result.err());

    let computation = result.unwrap();

    // Verify computation details
    assert_eq!(computation.field, TickType::Bid, "computation.field");
    assert_eq!(computation.tick_attribute, None, "computation.tick_attribute");
    assert_eq!(computation.implied_volatility, Some(0.3), "computation.implied_volatility");
    assert_eq!(computation.delta, Some(0.65), "computation.delta");
    assert_eq!(computation.option_price, Some(5.75), "computation.option_price");
    assert_eq!(computation.present_value_dividend, Some(0.5), "computation.present_value_dividend");
    assert_eq!(computation.gamma, Some(0.05), "computation.gamma");
    assert_eq!(computation.vega, Some(0.15), "computation.vega");
    assert_eq!(computation.theta, Some(0.01), "computation.theta");
    assert_eq!(computation.underlying_price, Some(145.0), "computation.underlying_price");

    // Test error case with server version too old
    let old_client = Client::stubbed(
        Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        }),
        server_versions::REQ_CALC_OPTION_PRICE - 1,
    );

    let error_result = old_client.calculate_option_price(&contract, volatility, underlying_price);
    assert!(error_result.is_err(), "Should fail with old server version");
}

#[test]
fn test_calculate_implied_volatility() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Option computation format: message_type, request_id, tick_type, tick_attribute, implied_vol, delta, option_price, pv_dividend, gamma, vega, theta, underlying_price
            "21|9000|13|1|0.25|0.60|7.5|0.45|0.04|0.12|0.02|148.0|".to_string(),
        ],
    });

    let client = Client::stubbed(message_bus, server_versions::REQ_CALC_IMPLIED_VOLAT);

    // Create test option contract
    let contract = Contract::option("AAPL", "20231215", 150.0, "C");

    // Test input parameters
    let option_price = 7.5;
    let underlying_price = 148.0;

    // Call the calculate_implied_volatility function
    let result = client.calculate_implied_volatility(&contract, option_price, underlying_price);

    // Check request encoding
    let request_messages = client.message_bus.request_messages();

    // Request format: message_type, version, request_id, contract fields..., option_price, underlying_price, empty
    assert!(
        request_messages[0].encode_simple().contains("0|AAPL|OPT|20231215|150|C||SMART"),
        "Unexpected request message format"
    );

    // Verify result
    assert!(result.is_ok(), "failed to calculate implied volatility: {:?}", result.err());

    let computation = result.unwrap();

    // Verify computation details
    assert_eq!(computation.field, TickType::Bid, "computation.field");
    assert_eq!(computation.tick_attribute, None, "computation.tick_attribute");
    assert_eq!(computation.implied_volatility, Some(0.25), "computation.implied_volatility");
    assert_eq!(computation.delta, Some(0.60), "computation.delta");
    assert_eq!(computation.option_price, Some(7.5), "computation.option_price");
    assert_eq!(computation.present_value_dividend, Some(0.45), "computation.present_value_dividend");
    assert_eq!(computation.gamma, Some(0.04), "computation.gamma");
    assert_eq!(computation.vega, Some(0.12), "computation.vega");
    assert_eq!(computation.theta, Some(0.02), "computation.theta");
    assert_eq!(computation.underlying_price, Some(148.0), "computation.underlying_price");

    // Test error case with server version too old
    let old_client = Client::stubbed(
        Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        }),
        server_versions::REQ_CALC_IMPLIED_VOLAT - 1,
    );

    let error_result = old_client.calculate_implied_volatility(&contract, option_price, underlying_price);
    assert!(error_result.is_err(), "Should fail with old server version");
}

#[test]
fn test_option_chain() {
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Security definition option parameter format: message_type, request_id, exchange, underlying_contract_id, trading_class, multiplier, expirations_count, expirations, strikes_count, strikes
            "75|9000|CBOE|12345|AAPL|100|3|20230616|20230915|20231215|3|140|150|160|".to_string(),
            // Security definition option parameter end format: message_type, request_id
            "76|9000|".to_string(),
        ],
    });

    let client = Client::stubbed(message_bus, server_versions::SEC_DEF_OPT_PARAMS_REQ);

    // Call the option_chain function
    let symbol = "AAPL";
    let exchange = "CBOE";
    let security_type = SecurityType::Stock;
    let contract_id = 12345;

    let result = client.option_chain(symbol, exchange, security_type, contract_id);

    // Check request encoding
    let request_messages = client.message_bus.request_messages();

    // Request format: message_type, request_id, underlying_symbol, exchange, underlying_security_type, contract_id
    assert_eq!(request_messages[0].encode_simple(), "78|9000|AAPL|CBOE|STK|12345|");

    // Verify result
    assert!(result.is_ok(), "failed to get option chain: {:?}", result.err());

    let subscription = result.unwrap();

    // Collect all items from the subscription
    let mut option_chains = Vec::new();
    for chain in &subscription {
        option_chains.push(chain);
    }

    if let Some(err) = subscription.error() {
        panic!("Expected no error in subscription: {:?}", err);
    }

    // We should have received one option chain
    assert_eq!(option_chains.len(), 1, "Expected 1 option chain");

    // Verify option chain details
    let chain = &option_chains[0];
    assert_eq!(chain.underlying_contract_id, 12345, "chain.underlying_contract_id");
    assert_eq!(chain.trading_class, "AAPL", "chain.trading_class");
    assert_eq!(chain.multiplier, "100", "chain.multiplier");
    assert_eq!(chain.exchange, "CBOE", "chain.exchange");

    // Verify expirations
    assert_eq!(chain.expirations.len(), 3, "chain.expirations.len()");
    assert_eq!(chain.expirations[0], "20230616", "chain.expirations[0]");
    assert_eq!(chain.expirations[1], "20230915", "chain.expirations[1]");
    assert_eq!(chain.expirations[2], "20231215", "chain.expirations[2]");

    // Verify strikes
    assert_eq!(chain.strikes.len(), 3, "chain.strikes.len()");
    assert_eq!(chain.strikes[0], 140.0, "chain.strikes[0]");
    assert_eq!(chain.strikes[1], 150.0, "chain.strikes[1]");
    assert_eq!(chain.strikes[2], 160.0, "chain.strikes[2]");

    // Test error case with server version too old
    let old_client = Client::stubbed(
        Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        }),
        server_versions::SEC_DEF_OPT_PARAMS_REQ - 1,
    );

    let error_result = old_client.option_chain(symbol, exchange, SecurityType::Stock, contract_id);
    assert!(error_result.is_err(), "Should fail with old server version");
}

#[test]
fn test_contract_details_errors() {
    // Test case 1: Error message from server
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Error format: message_type, request_id, error_code, error_message
            "3|9000|200|No security definition has been found for the request|".to_string(),
        ],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let contract = Contract::stock("INVALID_SYMBOL");

    let result = client.contract_details(&contract);

    // Verify that the error is correctly propagated
    assert!(result.is_err(), "Expected error for invalid symbol");
    if let Err(err) = result {
        assert!(
            format!("{:?}", err).contains("No security definition"),
            "Error message should contain 'No security definition'"
        );
    }

    // Test case 2: Unexpected end of stream
    let message_bus = Arc::new(MessageBusStub{
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Contract data without end message
            "10|9001|TSLA|STK||0||SMART|USD|TSLA|NMS|NMS|76792991|0.01||ACTIVETIM|SMART|1|0|TESLA INC|NASDAQ||Consumer|Auto|Cars|US/Eastern|09:30-16:00|09:30-16:00|||1|ISIN|US88160R1014|1|||26|20230616||1|1|100|".to_string(),
        ]
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let contract = Contract::stock("TSLA");

    let result = client.contract_details(&contract);

    // Verify that the unexpected end of stream error is correctly propagated
    assert!(result.is_err(), "Expected error for unexpected end of stream");
    if let Err(err) = result {
        assert!(
            format!("{:?}", err).contains("UnexpectedEndOfStream"),
            "Error should be UnexpectedEndOfStream"
        );
    }

    // Test case 3: Unexpected response message
    let message_bus = Arc::new(MessageBusStub {
        request_messages: RwLock::new(vec![]),
        response_messages: vec![
            // Unexpected message type
            "51|9001|CBOE|12345|AAPL|100|3|20230616,20230915,20231215|3|140,150,160|".to_string(),
        ],
    });

    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let contract = Contract::stock("AAPL");

    let result = client.contract_details(&contract);

    // Verify that the unexpected response error is correctly propagated
    assert!(result.is_err(), "Expected error for unexpected response");
    if let Err(err) = result {
        assert!(format!("{:?}", err).contains("UnexpectedResponse"), "Error should be UnexpectedResponse");
    }
}

#[test]
fn test_is_bag() {
    // Test with a regular stock contract (not a bag/spread)
    let stock_contract = Contract::stock("AAPL");
    assert!(!stock_contract.is_bag(), "Stock contract should not be a bag");

    // Test with a regular option contract (not a bag/spread)
    let option_contract = Contract::option("AAPL", "20231215", 150.0, "C");
    assert!(!option_contract.is_bag(), "Option contract should not be a bag");

    // Test with a futures contract (not a bag/spread)
    let futures_contract = Contract::futures("ES");
    assert!(!futures_contract.is_bag(), "Futures contract should not be a bag");

    // Test with a contract that is a bag/spread
    let mut spread_contract = Contract::default();
    spread_contract.security_type = SecurityType::Spread;
    assert!(spread_contract.is_bag(), "Spread contract should be a bag");

    // Test with an explicitly set BAG security type
    let mut bag_contract = Contract::default();
    bag_contract.security_type = SecurityType::from("BAG");
    assert!(bag_contract.is_bag(), "BAG contract should be a bag");

    // Test with combo legs
    let mut combo_contract = Contract::default();
    combo_contract.security_type = SecurityType::Spread;
    combo_contract.combo_legs = vec![
        ComboLeg {
            contract_id: 12345,
            ratio: 1,
            action: "BUY".to_string(),
            exchange: "SMART".to_string(),
            ..Default::default()
        },
        ComboLeg {
            contract_id: 67890,
            ratio: 1,
            action: "SELL".to_string(),
            exchange: "SMART".to_string(),
            ..Default::default()
        },
    ];
    assert!(combo_contract.is_bag(), "Contract with combo legs should be a bag");
}

#[test]
fn test_contract_builder_new() {
    let builder = ContractBuilder::new();

    // All fields should be None initially
    assert_eq!(builder.symbol, None);
    assert_eq!(builder.security_type, None);
    assert_eq!(builder.exchange, None);
    assert_eq!(builder.currency, None);
}

#[test]
fn test_contract_builder_field_setters() {
    let builder = ContractBuilder::new()
        .contract_id(12345)
        .symbol("AAPL")
        .security_type(SecurityType::Stock)
        .exchange("NASDAQ")
        .currency("USD")
        .strike(150.0)
        .right("C")
        .last_trade_date_or_contract_month("20231215")
        .multiplier("100")
        .local_symbol("AAPL_123")
        .primary_exchange("NASDAQ")
        .trading_class("AAPL")
        .include_expired(true)
        .security_id_type("ISIN")
        .security_id("US0378331005")
        .combo_legs_description("Test combo")
        .issuer_id("ISSUER123")
        .description("Apple Inc.");

    assert_eq!(builder.contract_id, Some(12345));
    assert_eq!(builder.symbol, Some("AAPL".to_string()));
    assert_eq!(builder.security_type, Some(SecurityType::Stock));
    assert_eq!(builder.exchange, Some("NASDAQ".to_string()));
    assert_eq!(builder.currency, Some("USD".to_string()));
    assert_eq!(builder.strike, Some(150.0));
    assert_eq!(builder.right, Some("C".to_string()));
    assert_eq!(builder.last_trade_date_or_contract_month, Some("20231215".to_string()));
    assert_eq!(builder.multiplier, Some("100".to_string()));
    assert_eq!(builder.local_symbol, Some("AAPL_123".to_string()));
    assert_eq!(builder.primary_exchange, Some("NASDAQ".to_string()));
    assert_eq!(builder.trading_class, Some("AAPL".to_string()));
    assert_eq!(builder.include_expired, Some(true));
    assert_eq!(builder.security_id_type, Some("ISIN".to_string()));
    assert_eq!(builder.security_id, Some("US0378331005".to_string()));
    assert_eq!(builder.combo_legs_description, Some("Test combo".to_string()));
    assert_eq!(builder.issuer_id, Some("ISSUER123".to_string()));
    assert_eq!(builder.description, Some("Apple Inc.".to_string()));
}

#[test]
fn test_contract_builder_stock() {
    let builder = ContractBuilder::stock("AAPL", "NASDAQ", "USD");

    assert_eq!(builder.symbol, Some("AAPL".to_string()));
    assert_eq!(builder.security_type, Some(SecurityType::Stock));
    assert_eq!(builder.exchange, Some("NASDAQ".to_string()));
    assert_eq!(builder.currency, Some("USD".to_string()));
}

#[test]
fn test_contract_builder_futures() {
    let builder = ContractBuilder::futures("ES", "CME", "USD");

    assert_eq!(builder.symbol, Some("ES".to_string()));
    assert_eq!(builder.security_type, Some(SecurityType::Future));
    assert_eq!(builder.exchange, Some("CME".to_string()));
    assert_eq!(builder.currency, Some("USD".to_string()));
}

#[test]
fn test_contract_builder_crypto() {
    let builder = ContractBuilder::crypto("BTC", "PAXOS", "USD");

    assert_eq!(builder.symbol, Some("BTC".to_string()));
    assert_eq!(builder.security_type, Some(SecurityType::Crypto));
    assert_eq!(builder.exchange, Some("PAXOS".to_string()));
    assert_eq!(builder.currency, Some("USD".to_string()));
}

#[test]
fn test_contract_builder_option() {
    let builder = ContractBuilder::option("AAPL", "SMART", "USD");

    assert_eq!(builder.symbol, Some("AAPL".to_string()));
    assert_eq!(builder.security_type, Some(SecurityType::Option));
    assert_eq!(builder.exchange, Some("SMART".to_string()));
    assert_eq!(builder.currency, Some("USD".to_string()));
}

#[test]
fn test_contract_builder_build_stock_success() {
    let contract = ContractBuilder::stock("AAPL", "NASDAQ", "USD").contract_id(12345).build().unwrap();

    assert_eq!(contract.symbol, "AAPL");
    assert_eq!(contract.security_type, SecurityType::Stock);
    assert_eq!(contract.exchange, "NASDAQ");
    assert_eq!(contract.currency, "USD");
    assert_eq!(contract.contract_id, 12345);
    assert_eq!(contract.strike, 0.0);
    assert_eq!(contract.right, "");
    assert_eq!(contract.last_trade_date_or_contract_month, "");
    assert!(!contract.include_expired);
}

#[test]
fn test_contract_builder_build_option_success() {
    let contract = ContractBuilder::option("AAPL", "SMART", "USD")
        .strike(150.0)
        .right("C")
        .last_trade_date_or_contract_month("20231215")
        .build()
        .unwrap();

    assert_eq!(contract.symbol, "AAPL");
    assert_eq!(contract.security_type, SecurityType::Option);
    assert_eq!(contract.exchange, "SMART");
    assert_eq!(contract.currency, "USD");
    assert_eq!(contract.strike, 150.0);
    assert_eq!(contract.right, "C");
    assert_eq!(contract.last_trade_date_or_contract_month, "20231215");
}

#[test]
fn test_contract_builder_build_futures_success() {
    let contract = ContractBuilder::futures("ES", "CME", "USD")
        .last_trade_date_or_contract_month("202312")
        .build()
        .unwrap();

    assert_eq!(contract.symbol, "ES");
    assert_eq!(contract.security_type, SecurityType::Future);
    assert_eq!(contract.exchange, "CME");
    assert_eq!(contract.currency, "USD");
    assert_eq!(contract.last_trade_date_or_contract_month, "202312");
}

#[test]
fn test_contract_builder_build_missing_symbol() {
    let result = ContractBuilder::new().build();

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "error occurred: Symbol is required");
}

#[test]
fn test_contract_builder_build_option_missing_strike() {
    let result = ContractBuilder::option("AAPL", "SMART", "USD")
        .right("C")
        .last_trade_date_or_contract_month("20231215")
        .build();

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "error occurred: Strike price is required for options");
}

#[test]
fn test_contract_builder_build_option_missing_right() {
    let result = ContractBuilder::option("AAPL", "SMART", "USD")
        .strike(150.0)
        .last_trade_date_or_contract_month("20231215")
        .build();

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "error occurred: Right (P for PUT or C for CALL) is required for options"
    );
}

#[test]
fn test_contract_builder_build_option_missing_expiration() {
    let result = ContractBuilder::option("AAPL", "SMART", "USD").strike(150.0).right("C").build();

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "error occurred: Expiration date is required for options");
}

#[test]
fn test_contract_builder_build_futures_missing_contract_month() {
    let result = ContractBuilder::futures("ES", "CME", "USD").build();

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "error occurred: Contract month is required for futures");
}

#[test]
fn test_contract_builder_build_futures_option_missing_contract_month() {
    let result = ContractBuilder::new()
        .symbol("ES")
        .security_type(SecurityType::FuturesOption)
        .exchange("CME")
        .currency("USD")
        .build();

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "error occurred: Contract month is required for futures");
}

#[test]
fn test_contract_builder_build_invalid_option_right() {
    let result = ContractBuilder::option("AAPL", "SMART", "USD")
        .strike(150.0)
        .right("INVALID")
        .last_trade_date_or_contract_month("20231215")
        .build();

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "error occurred: Option right must be P for PUT or C for CALL"
    );
}

#[test]
fn test_contract_builder_build_valid_option_rights() {
    let valid_rights = ["P", "C", "p", "c"];

    for right in &valid_rights {
        let result = ContractBuilder::option("AAPL", "SMART", "USD")
            .strike(150.0)
            .right(*right)
            .last_trade_date_or_contract_month("20231215")
            .build();

        assert!(result.is_ok(), "Right '{}' should be valid", right);
    }
}

#[test]
fn test_contract_builder_build_negative_strike() {
    let result = ContractBuilder::option("AAPL", "SMART", "USD")
        .strike(-10.0)
        .right("C")
        .last_trade_date_or_contract_month("20231215")
        .build();

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "error occurred: Strike price cannot be negative");
}

#[test]
fn test_contract_builder_combo_legs() {
    let combo_legs = vec![
        ComboLeg {
            contract_id: 12345,
            ratio: 1,
            action: "BUY".to_string(),
            exchange: "SMART".to_string(),
            ..Default::default()
        },
        ComboLeg {
            contract_id: 67890,
            ratio: 1,
            action: "SELL".to_string(),
            exchange: "SMART".to_string(),
            ..Default::default()
        },
    ];

    let contract = ContractBuilder::new()
        .symbol("SPREAD")
        .security_type(SecurityType::Spread)
        .combo_legs(combo_legs.clone())
        .build()
        .unwrap();

    assert_eq!(contract.combo_legs.len(), 2);
    assert_eq!(contract.combo_legs[0].contract_id, 12345);
    assert_eq!(contract.combo_legs[0].action, "BUY");
    assert_eq!(contract.combo_legs[1].contract_id, 67890);
    assert_eq!(contract.combo_legs[1].action, "SELL");
}

#[test]
fn test_contract_builder_delta_neutral_contract() {
    let delta_neutral = DeltaNeutralContract {
        contract_id: 12345,
        delta: 0.5,
        price: 100.0,
    };

    let contract = ContractBuilder::stock("AAPL", "NASDAQ", "USD")
        .delta_neutral_contract(delta_neutral.clone())
        .build()
        .unwrap();

    assert!(contract.delta_neutral_contract.is_some());
    let delta_neutral_result = contract.delta_neutral_contract.unwrap();
    assert_eq!(delta_neutral_result.contract_id, 12345);
    assert_eq!(delta_neutral_result.delta, 0.5);
    assert_eq!(delta_neutral_result.price, 100.0);
}

#[test]
fn test_contract_builder_chaining() {
    // Test that builder methods can be chained fluently
    let contract = ContractBuilder::new()
        .symbol("TSLA")
        .security_type(SecurityType::Stock)
        .exchange("NASDAQ")
        .currency("USD")
        .contract_id(76792991)
        .primary_exchange("NASDAQ")
        .local_symbol("TSLA")
        .trading_class("TSLA")
        .description("Tesla Inc.")
        .build()
        .unwrap();

    assert_eq!(contract.symbol, "TSLA");
    assert_eq!(contract.security_type, SecurityType::Stock);
    assert_eq!(contract.exchange, "NASDAQ");
    assert_eq!(contract.currency, "USD");
    assert_eq!(contract.contract_id, 76792991);
    assert_eq!(contract.primary_exchange, "NASDAQ");
    assert_eq!(contract.local_symbol, "TSLA");
    assert_eq!(contract.trading_class, "TSLA");
    assert_eq!(contract.description, "Tesla Inc.");
}

#[test]
fn test_contract_builder_defaults() {
    // Test that unset fields get proper defaults
    let contract = ContractBuilder::new().symbol("TEST").build().unwrap();

    assert_eq!(contract.contract_id, 0);
    assert_eq!(contract.symbol, "TEST");
    assert_eq!(contract.security_type, SecurityType::Stock); // Default
    assert_eq!(contract.last_trade_date_or_contract_month, "");
    assert_eq!(contract.strike, 0.0);
    assert_eq!(contract.right, "");
    assert_eq!(contract.multiplier, "");
    assert_eq!(contract.exchange, "");
    assert_eq!(contract.currency, "");
    assert_eq!(contract.local_symbol, "");
    assert_eq!(contract.primary_exchange, "");
    assert_eq!(contract.trading_class, "");
    assert!(!contract.include_expired);
    assert_eq!(contract.security_id_type, "");
    assert_eq!(contract.security_id, "");
    assert_eq!(contract.combo_legs_description, "");
    assert!(contract.combo_legs.is_empty());
    assert!(contract.delta_neutral_contract.is_none());
    assert_eq!(contract.issuer_id, "");
    assert_eq!(contract.description, "");
}
