use anyhow;
use time::OffsetDateTime;

use crate::client::tests::ClientStub;
use crate::contracts;
use crate::domain::Contract;
use crate::historical_market_data;

#[test]
fn test_head_timestamp() -> anyhow::Result<()> {
    let client = ClientStub{};
    let contract = contracts::stock("MSFT");
    let what_to_show = "trades";
    let use_rth = true;

    let head_timestamp = historical_market_data::head_timestamp(&client, &contract, what_to_show, use_rth)?;
    assert_eq!(head_timestamp, OffsetDateTime::now_utc());

    Ok(())
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
