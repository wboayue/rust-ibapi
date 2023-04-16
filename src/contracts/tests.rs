use std::cell::RefCell;

use super::*;

use crate::stubs::MessageBusStub;

#[test]
fn request_stock_contract_details() {
    let message_bus = RefCell::new(Box::new(MessageBusStub{
        request_messages: RefCell::new(vec![]),
        response_messages: vec![
            "10|9001|TSLA|STK||0||SMART|USD|TSLA|NMS|NMS|76792991|0.01||ACTIVETIM,AD,ADJUST,ALERT,ALGO,ALLOC,AON,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DARKONLY,DARKPOLL,DAY,DEACT,DEACTDIS,DEACTEOD,DIS,DUR,GAT,GTC,GTD,GTT,HID,IBKRATS,ICE,IMB,IOC,LIT,LMT,LOC,MIDPX,MIT,MKT,MOC,MTL,NGCOMB,NODARK,NONALGO,OCA,OPG,OPGREROUT,PEGBENCH,PEGMID,POSTATS,POSTONLY,PREOPGRTH,PRICECHK,REL,REL2MID,RELPCTOFS,RPI,RTH,SCALE,SCALEODD,SCALERST,SIZECHK,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,SWEEP,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF|SMART,AMEX,NYSE,CBOE,PHLX,ISE,CHX,ARCA,ISLAND,DRCTEDGE,BEX,BATS,EDGEA,CSFBALGO,JEFFALGO,BYX,IEX,EDGX,FOXRIVER,PEARL,NYSENAT,LTSE,MEMX,PSX|1|0|TESLA INC|NASDAQ||Consumer, Cyclical|Auto Manufacturers|Auto-Cars/Light Trucks|US/Eastern|20221229:0400-20221229:2000;20221230:0400-20221230:2000;20221231:CLOSED;20230101:CLOSED;20230102:CLOSED;20230103:0400-20230103:2000|20221229:0930-20221229:1600;20221230:0930-20221230:1600;20221231:CLOSED;20230101:CLOSED;20230102:CLOSED;20230103:0930-20230103:1600|||1|ISIN|US88160R1014|1|||26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26||COMMON|1|1|100||".to_string(),
            "10|9001|TSLA|STK||0||AMEX|USD|TSLA|NMS|NMS|76792991|0.01||ACTIVETIM,AD,ADJUST,ALERT,ALLOC,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DAY,DEACT,DEACTDIS,DEACTEOD,GAT,GTC,GTD,GTT,HID,IOC,LIT,LMT,MIT,MKT,MTL,NGCOMB,NONALGO,OCA,PEGBENCH,SCALE,SCALERST,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF|SMART,AMEX,NYSE,CBOE,PHLX,ISE,CHX,ARCA,ISLAND,DRCTEDGE,BEX,BATS,EDGEA,CSFBALGO,JEFFALGO,BYX,IEX,EDGX,FOXRIVER,PEARL,NYSENAT,LTSE,MEMX,PSX|1|0|TESLA INC|NASDAQ||Consumer, Cyclical|Auto Manufacturers|Auto-Cars/Light Trucks|US/Eastern|20221229:0700-20221229:2000;20221230:0700-20221230:2000;20221231:CLOSED;20230101:CLOSED;20230102:CLOSED;20230103:0700-20230103:2000|20221229:0700-20221229:2000;20221230:0700-20221230:2000;20221231:CLOSED;20230101:CLOSED;20230102:CLOSED;20230103:0700-20230103:2000|||1|ISIN|US88160R1014|1|||26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26||COMMON|1|1|100||".to_string(),
            "52|1|9001||".to_string(),
        ]
    }));

    let mut client = Client::stubbed(message_bus, server_versions::SIZE_RULES);

    let contract = Contract::stock("TSLA");

    let results = client.contract_details(&contract);

    let request_messages = client.message_bus.borrow().request_messages();

    assert_eq!(request_messages[0].encode_simple(), "9|8|9000|0|TSLA|STK||0|||SMART||USD|||0|||");

    assert!(results.is_ok(), "failed to encode request: {:?}", results.err());

    let contracts: Vec<ContractDetails> = results.unwrap().collect();
    assert_eq!(2, contracts.len());

    assert_eq!(contracts[0].contract.exchange, "SMART");
    assert_eq!(contracts[1].contract.exchange, "AMEX");

    assert_eq!(contracts[0].contract.symbol, "TSLA");
    assert_eq!(contracts[0].contract.security_type, SecurityType::Stock);
    assert_eq!(contracts[0].contract.currency, "USD");
    assert_eq!(contracts[0].contract.contract_id, 76792991);
    assert_eq!(contracts[0].order_types, "ACTIVETIM,AD,ADJUST,ALERT,ALGO,ALLOC,AON,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DARKONLY,DARKPOLL,DAY,DEACT,DEACTDIS,DEACTEOD,DIS,DUR,GAT,GTC,GTD,GTT,HID,IBKRATS,ICE,IMB,IOC,LIT,LMT,LOC,MIDPX,MIT,MKT,MOC,MTL,NGCOMB,NODARK,NONALGO,OCA,OPG,OPGREROUT,PEGBENCH,PEGMID,POSTATS,POSTONLY,PREOPGRTH,PRICECHK,REL,REL2MID,RELPCTOFS,RPI,RTH,SCALE,SCALEODD,SCALERST,SIZECHK,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,SWEEP,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF");
    assert_eq!(
        contracts[0].valid_exchanges,
        "SMART,AMEX,NYSE,CBOE,PHLX,ISE,CHX,ARCA,ISLAND,DRCTEDGE,BEX,BATS,EDGEA,CSFBALGO,JEFFALGO,BYX,IEX,EDGX,FOXRIVER,PEARL,NYSENAT,LTSE,MEMX,PSX"
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
        "20221229:0400-20221229:2000;20221230:0400-20221230:2000;20221231:CLOSED;20230101:CLOSED;20230102:CLOSED;20230103:0400-20230103:2000"
    );
    assert_eq!(
        contracts[0].liquid_hours,
        "20221229:0930-20221229:1600;20221230:0930-20221230:1600;20221231:CLOSED;20230101:CLOSED;20230102:CLOSED;20230103:0930-20230103:1600"
    );
    assert_eq!(contracts[0].ev_rule, "");
    assert_eq!(contracts[0].ev_multiplier, 0.0);
    assert_eq!(contracts[0].sec_id_list.len(), 1);
    assert_eq!(contracts[0].sec_id_list[0].tag, "ISIN");
    assert_eq!(contracts[0].sec_id_list[0].value, "US88160R1014");
    assert_eq!(contracts[0].agg_group, 1);
    assert_eq!(
        contracts[0].market_rule_ids,
        "26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26"
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
