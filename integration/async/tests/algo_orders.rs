use futures::StreamExt;
use ibapi::contracts::Contract;
use ibapi::orders::builder::{
    accu_distr, accumulate_distribute, adaptive, arrival_price, balance_impact_risk, close_price, dark_ice, minimise_impact, pct_vol, pct_vol_price,
    pct_vol_size, pct_vol_time, twap, vwap, AdaptivePriority, AlgoParams, RiskAversion, TwapStrategyType,
};
use ibapi::orders::{Order, PlaceOrder};
use ibapi::subscriptions::SubscriptionItem;
use ibapi::{Client, Error};
use ibapi_test::{rate_limit, yyyymmdd_today, ClientId, GATEWAY};
use serial_test::serial;
use tokio::time::{timeout, Duration};

async fn connect() -> (Client, ClientId) {
    let client_id = ClientId::get();
    rate_limit();
    let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");
    (client, client_id)
}

/// Build an algo-bearing order via the public OrderBuilder path. Far-from-market
/// limit so it never fills.
fn algo_order(client: &Client, contract: &Contract, algo: AlgoParams) -> Order {
    client
        .order(contract)
        .buy(1)
        .limit(1.0)
        .algo(algo)
        .build_order()
        .expect("order builder failed")
}

/// Submits the order, waits for TWS to acknowledge (OrderStatus / OpenOrder /
/// non-rejection Notice), then cancels. Code 201 (hard rejection) fails the
/// test — that's what would catch a malformed algo encoding.
async fn submit_and_cleanup(client: &Client, contract: &Contract, order: &Order) {
    rate_limit();
    let order_id = client.next_order_id();
    let mut subscription = client.place_order(order_id, contract, order).await.expect("place_order failed");

    let mut acknowledged = false;
    while let Ok(Some(result)) = timeout(Duration::from_secs(5), subscription.next()).await {
        match result {
            Ok(SubscriptionItem::Data(PlaceOrder::OrderStatus(_) | PlaceOrder::OpenOrder(_))) => {
                acknowledged = true;
                break;
            }
            Ok(SubscriptionItem::Notice(notice)) => {
                if notice.message.contains("rejected") {
                    panic!("TWS rejected algo order: {}", notice.message);
                }
                acknowledged = true;
                break;
            }
            Ok(SubscriptionItem::Data(_)) => continue,
            Err(Error::Message(201, msg)) => panic!("TWS rejected algo order [201]: {msg}"),
            Err(Error::Message(_, _)) => {
                acknowledged = true;
                break;
            }
            Err(e) => panic!("subscription error: {e}"),
        }
    }
    assert!(acknowledged, "no acknowledgement from TWS within timeout");

    rate_limit();
    let _ = client.cancel_order(order_id, "").await;
}

// === Existing core 4 ===

#[tokio::test]
#[serial(orders)]
async fn submits_vwap() {
    let (client, _id) = connect().await;
    let contract = Contract::stock("AAPL").build();
    let algo = vwap()
        .max_pct_vol(0.2)
        .start_time("09:30:00 US/Eastern")
        .end_time("16:00:00 US/Eastern")
        .build()
        .unwrap();
    let order = algo_order(&client, &contract, algo);
    submit_and_cleanup(&client, &contract, &order).await;
}

#[tokio::test]
#[serial(orders)]
async fn submits_twap() {
    let (client, _id) = connect().await;
    let contract = Contract::stock("AAPL").build();
    let algo = twap()
        .strategy_type(TwapStrategyType::Marketable)
        .start_time("09:30:00 US/Eastern")
        .end_time("16:00:00 US/Eastern")
        .build()
        .unwrap();
    let order = algo_order(&client, &contract, algo);
    submit_and_cleanup(&client, &contract, &order).await;
}

#[tokio::test]
#[serial(orders)]
async fn submits_pct_vol() {
    let (client, _id) = connect().await;
    let contract = Contract::stock("AAPL").build();
    let algo = pct_vol()
        .pct_vol(0.1)
        .start_time("09:30:00 US/Eastern")
        .end_time("16:00:00 US/Eastern")
        .build()
        .unwrap();
    let order = algo_order(&client, &contract, algo);
    submit_and_cleanup(&client, &contract, &order).await;
}

#[tokio::test]
#[serial(orders)]
async fn submits_arrival_price() {
    let (client, _id) = connect().await;
    let contract = Contract::stock("AAPL").build();
    let algo = arrival_price()
        .max_pct_vol(0.1)
        .risk_aversion(RiskAversion::Neutral)
        .start_time("09:30:00 US/Eastern")
        .end_time("16:00:00 US/Eastern")
        .build()
        .unwrap();
    let order = algo_order(&client, &contract, algo);
    submit_and_cleanup(&client, &contract, &order).await;
}

// === Priority 1 ===

#[tokio::test]
#[serial(orders)]
async fn submits_adaptive() {
    let (client, _id) = connect().await;
    let contract = Contract::stock("AAPL").build();
    let algo = adaptive().priority(AdaptivePriority::Normal).build().unwrap();
    let order = algo_order(&client, &contract, algo);
    submit_and_cleanup(&client, &contract, &order).await;
}

#[tokio::test]
#[serial(orders)]
async fn submits_close_price() {
    let (client, _id) = connect().await;
    let contract = Contract::stock("AAPL").build();
    let algo = close_price()
        .max_pct_vol(0.2)
        .risk_aversion(RiskAversion::Neutral)
        .start_time("15:30:00 US/Eastern")
        .force_completion(false)
        .build()
        .unwrap();
    let order = algo_order(&client, &contract, algo);
    submit_and_cleanup(&client, &contract, &order).await;
}

#[tokio::test]
#[serial(orders)]
async fn submits_dark_ice() {
    let (client, _id) = connect().await;
    let contract = Contract::stock("AAPL").build();
    let algo = dark_ice()
        .display_size(100)
        .start_time("09:30:00 US/Eastern")
        .end_time("16:00:00 US/Eastern")
        .build()
        .unwrap();
    let order = algo_order(&client, &contract, algo);
    submit_and_cleanup(&client, &contract, &order).await;
}

// === Priority 2 ===

#[tokio::test]
#[serial(orders)]
async fn submits_accumulate_distribute() {
    let (client, _id) = connect().await;
    let contract = Contract::stock("AAPL").build();
    let today = yyyymmdd_today();
    let algo = accumulate_distribute()
        .component_size(100)
        .time_between_orders(60)
        .randomize_time_20(true)
        .randomize_size_55(true)
        .wait_for_fill(true)
        .active_time_start(format!("{today}-04:00:00 US/Eastern"))
        .active_time_end(format!("{today}-20:00:00 US/Eastern"))
        .build()
        .unwrap();
    let order = algo_order(&client, &contract, algo);
    submit_and_cleanup(&client, &contract, &order).await;
}

#[tokio::test]
#[serial(orders)]
async fn submits_balance_impact_risk() {
    let (client, _id) = connect().await;
    let contract = Contract::stock("AAPL").build();
    let algo = balance_impact_risk()
        .max_pct_vol(0.2)
        .risk_aversion(RiskAversion::Neutral)
        .force_completion(false)
        .build()
        .unwrap();
    let order = algo_order(&client, &contract, algo);
    submit_and_cleanup(&client, &contract, &order).await;
}

#[tokio::test]
#[serial(orders)]
async fn submits_minimise_impact() {
    let (client, _id) = connect().await;
    let contract = Contract::stock("AAPL").build();
    let algo = minimise_impact().max_pct_vol(0.2).build().unwrap();
    let order = algo_order(&client, &contract, algo);
    submit_and_cleanup(&client, &contract, &order).await;
}

// === Priority 3: PctVol variants ===

#[tokio::test]
#[serial(orders)]
async fn submits_pct_vol_price() {
    let (client, _id) = connect().await;
    let contract = Contract::stock("AAPL").build();
    let algo = pct_vol_price()
        .pct_vol(0.2)
        .delta_pct_vol(0.1)
        .min_pct_vol_4_px(0.1)
        .max_pct_vol_4_px(0.4)
        .start_time("09:30:00 US/Eastern")
        .end_time("16:00:00 US/Eastern")
        .build()
        .unwrap();
    let order = algo_order(&client, &contract, algo);
    submit_and_cleanup(&client, &contract, &order).await;
}

#[tokio::test]
#[serial(orders)]
async fn submits_pct_vol_size() {
    let (client, _id) = connect().await;
    let contract = Contract::stock("AAPL").build();
    let algo = pct_vol_size()
        .start_pct_vol(0.1)
        .end_pct_vol(0.4)
        .start_time("09:30:00 US/Eastern")
        .end_time("16:00:00 US/Eastern")
        .build()
        .unwrap();
    let order = algo_order(&client, &contract, algo);
    submit_and_cleanup(&client, &contract, &order).await;
}

#[tokio::test]
#[serial(orders)]
async fn submits_pct_vol_time() {
    let (client, _id) = connect().await;
    let contract = Contract::stock("AAPL").build();
    let algo = pct_vol_time()
        .start_pct_vol(0.1)
        .end_pct_vol(0.4)
        .start_time("09:30:00 US/Eastern")
        .end_time("16:00:00 US/Eastern")
        .build()
        .unwrap();
    let order = algo_order(&client, &contract, algo);
    submit_and_cleanup(&client, &contract, &order).await;
}

// === AccuDistr ===

#[tokio::test]
#[serial(orders)]
async fn submits_accu_distr() {
    let (client, _id) = connect().await;
    let contract = Contract::stock("AAPL").build();
    let today = yyyymmdd_today();
    let algo = accu_distr()
        .time_between_orders(60)
        .route_order_type("LMT")
        .component_size(100)
        .active_time_start(format!("{today}-04:00:00"))
        .active_time_end(format!("{today}-20:00:00"))
        .active_time_tz("US/Eastern")
        .build()
        .unwrap();
    let order = algo_order(&client, &contract, algo);
    submit_and_cleanup(&client, &contract, &order).await;
}
