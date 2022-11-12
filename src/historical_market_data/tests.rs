use anyhow::{anyhow};
use time::OffsetDateTime;

use crate::client::tests::ClientStub;
use crate::contracts;
use crate::domain::Contract;
use crate::historical_market_data;

#[test]
fn test_head_timestamp() {
    let client = ClientStub{};
    let contract = contracts::stock("MSFT");
    let what_to_show = "trades";
    let use_rth = true;

    let result = historical_market_data::head_timestamp(&client, &contract, what_to_show, use_rth);

    match result {
        Err(error) => !
            assert_eq!(error, anyhow!(""))
        Ok(head_timestamp) =>
            assert_eq!(head_timestamp, OffsetDateTime::now_utc())
    };
}

#[test]
fn histogram_data() {
    let result = 2 + 2;
    assert_eq!(result, 4);
}

#[test]
fn historical_data() {
    let result = 2 + 2;
    assert_eq!(result, 4);
}
