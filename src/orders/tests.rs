use super::*;
use crate::client::stub::ClientStub;
use crate::contracts::Contract;

#[test]
fn place_order() {
    let mut client = ClientStub::new(server_versions::SIZE_RULES);

    client.response_messages = vec![
    ];

    let contract = Contract::stock("TSLA");

    let order_id = 12;
    let order = order_builder::market_order(super::Action::Buy, 150.0);

    println!("contract: {contract:?}, order: {order:?}");

    let results = super::place_order(&mut client, order_id, &contract, &order);

    println!("order: {results:?}");

    // assert_eq!(
    //     client.request_messages[0],
    //     "9|8|3000|0|TSLA|STK||0|||SMART||USD|||0|||"
    // );
}
