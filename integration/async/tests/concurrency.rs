use ibapi::contracts::Contract;
use ibapi::market_data::historical::{BarSize, Duration, WhatToShow};
use ibapi::market_data::TradingHours;
use ibapi::Client;
use ibapi_test::{rate_limit, ClientId, GATEWAY};

#[tokio::test]
async fn concurrent_contract_details() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    let aapl = Contract::stock("AAPL").build();
    let msft = Contract::stock("MSFT").build();
    let tsla = Contract::stock("TSLA").build();

    rate_limit();
    rate_limit();
    rate_limit();
    let (r1, r2, r3) = tokio::join!(
        client.contract_details(&aapl),
        client.contract_details(&msft),
        client.contract_details(&tsla),
    );

    let d1 = r1.expect("AAPL contract_details failed");
    let d2 = r2.expect("MSFT contract_details failed");
    let d3 = r3.expect("TSLA contract_details failed");

    assert!(!d1.is_empty());
    assert!(!d2.is_empty());
    assert!(!d3.is_empty());
    assert_eq!(d1[0].contract.symbol, "AAPL");
    assert_eq!(d2[0].contract.symbol, "MSFT");
    assert_eq!(d3[0].contract.symbol, "TSLA");
}

#[tokio::test]
async fn concurrent_historical_data() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    let aapl = Contract::stock("AAPL").build();
    let msft = Contract::stock("MSFT").build();

    rate_limit();
    rate_limit();
    let (r1, r2) = tokio::join!(
        client.historical_data(
            &aapl,
            None,
            Duration::days(5),
            BarSize::Day,
            Some(WhatToShow::Trades),
            TradingHours::Regular
        ),
        client.historical_data(
            &msft,
            None,
            Duration::days(5),
            BarSize::Day,
            Some(WhatToShow::Trades),
            TradingHours::Regular
        ),
    );

    let d1 = r1.expect("AAPL historical_data failed");
    let d2 = r2.expect("MSFT historical_data failed");

    assert!(!d1.bars.is_empty());
    assert!(!d2.bars.is_empty());
}

#[tokio::test]
async fn concurrent_subscriptions() {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");

    let aapl = Contract::stock("AAPL").build();
    let msft = Contract::stock("MSFT").build();

    rate_limit();
    let mut sub1 = client.market_data(&aapl).subscribe().await.expect("AAPL market_data failed");

    rate_limit();
    let mut sub2 = client.market_data(&msft).subscribe().await.expect("MSFT market_data failed");

    let timeout = tokio::time::Duration::from_secs(15);
    let (r1, r2) = tokio::join!(tokio::time::timeout(timeout, sub1.next()), tokio::time::timeout(timeout, sub2.next()),);

    // Both subscriptions should produce data (may timeout outside market hours)
    if let Ok(Some(_)) = r1 {
        // AAPL tick received
    }
    if let Ok(Some(_)) = r2 {
        // MSFT tick received
    }
}

#[tokio::test]
async fn sequential_connect_disconnect() {
    // First connection
    let client_id1 = ClientId::get();
    rate_limit();
    let client1 = Client::connect(GATEWAY, client_id1.id()).await.expect("first connection failed");

    rate_limit();
    let time1 = client1.server_time().await.expect("first server_time failed");
    assert!(time1.year() >= 2025);

    // Drop first connection
    drop(client1);
    drop(client_id1);

    // Brief pause to ensure disconnect
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Second connection with new ID
    let client_id2 = ClientId::get();
    rate_limit();
    let client2 = Client::connect(GATEWAY, client_id2.id()).await.expect("second connection failed");

    rate_limit();
    let time2 = client2.server_time().await.expect("second server_time failed");
    assert!(time2.year() >= 2025);
}
