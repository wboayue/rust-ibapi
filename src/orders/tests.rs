use super::*;
use crate::client::stub::ClientStub;
use crate::contracts::{Contract, contract_samples};

#[test]
fn place_market_order() {
    let mut client = ClientStub::new(server_versions::SIZE_RULES);

    client.response_messages = vec![];

    let order_id = 12;
    let contract = contract_samples::future_with_local_symbol();
    let order = order_builder::market_order(super::Action::Buy, 10.0);

    let results = super::place_order(&mut client, order_id, &contract, &order);

    assert_eq!(
        client.request_messages[0],
        "3|12|0||FUT|202303|0|||EUREX||EUR|FGBL MAR 23||||BUY|10|MKT|||||||0||0|0|0|0|0|0|0|0||0||||||||0||0|0|||0|||0|0||||||||0|||||0|||||||||||0|||0|0|||0||0|0||0|0|0|0|0|0|||||||||0|0|0|0|||0|"
    );

    assert!(
        results.is_ok(),
        "failed to place order: {:?}",
        results.unwrap_err()
    );
}

#[test]
fn place_limit_order() {
    let mut client = ClientStub::new(server_versions::SIZE_RULES);

    client.response_messages = vec![];

    let order_id = 12;
    let contract = contract_samples::future_with_local_symbol();
    let order = order_builder::limit_order(super::Action::Buy, 10.0, 500.00);

    let results = super::place_order(&mut client, order_id, &contract, &order);

    assert_eq!(
        client.request_messages[0],
        "3|12|0||FUT|202303|0|||EUREX||EUR|FGBL MAR 23||||BUY|10|LMT|500||||||0||0|0|0|0|0|0|0|0||0||||||||0||0|0|||0|||0|0||||||||0|||||0|||||||||||0|||0|0|||0||0|0||0|0|0|0|0|0|||||||||0|0|0|0|||0|"
    );

    assert!(
        results.is_ok(),
        "failed to place order: {:?}",
        results.unwrap_err()
    );
}

#[test]
fn place_combo_market_order() {
    let mut client = ClientStub::new(server_versions::SIZE_RULES);

    client.response_messages = vec![];

    let order_id = 12;
    let contract = contract_samples::smart_future_combo_contract();
    let order = order_builder::combo_market_order(Action::Sell, 150.0, true);

    let results = super::place_order(&mut client, order_id, &contract, &order);

    assert_eq!(
        client.request_messages[0],
        "3|12|0|WTI|BAG||0|||SMART||USD|||||SELL|150|MKT|||||||0||0|0|0|0|0|0|0|0|2|55928698|1|BUY|IPE|0|0||0|55850663|1|SELL|IPE|0|0||0|0|1|NonGuaranteed|1||0||||||||0||0|0|||0|||0|0||||||||0|||||0|||||||||||0|||0|0|||0||0|0||0|0|0|0|0|0|||||||||0|0|0|0|||0|"
    );

    assert!(
        results.is_ok(),
        "failed to place order: {:?}",
        results.unwrap_err()
    );
}
