//! Placement checks for `SpreadBuilder` combos: each preset is sent to TWS as a what-if order,
//! which TWS validates without placing. Building a combo is unit-tested; only TWS can say it is
//! placeable (a combo without a symbol builds but draws 321, #1128).

use std::collections::BTreeMap;
use std::time::Duration;

use ibapi::contracts::{
    ButterflyLegs, CollarLegs, Contract, ExpirationDate, Expiry, HasLegs, IronCondorLegs, OptionRight, SecurityType, SpreadBuilder,
};
use ibapi::Client;
use ibapi_integration_async::connect;
use ibapi_test::{yyyymmdd_from_now, ClientId};

const SYMBOL: &str = "AAPL";

/// Strike in integer cents, so it can be a map key.
type StrikeCents = i64;

fn dollars(strike: StrikeCents) -> f64 {
    strike as f64 / 100.0
}

/// Strike → contract id for one expiry and right. Strike 0 asks TWS for every strike.
///
/// Assumes one trading class per strike (true for AAPL); a second class would silently replace
/// the first in the map.
async fn strikes(client: &Client, expiry: &str, right: OptionRight) -> BTreeMap<StrikeCents, i32> {
    let details = client
        .contract_details(&Contract::option(SYMBOL, expiry.parse().expect("YYYYMMDD expiry"), 0.0, right))
        .await
        .expect("option contract details failed");
    details
        .iter()
        .map(|d| ((d.contract.strike * 100.0).round() as i64, d.contract.contract_id))
        .collect()
}

/// The stock's contract id and its two nearest expirations at least two weeks out, so neither
/// expires mid-run.
async fn underlying_and_expiries(client: &Client) -> (i32, String, String) {
    let stock = client
        .contract_details(&Contract::stock(SYMBOL).build())
        .await
        .expect("stock contract details failed");
    let stock_id = stock[0].contract.contract_id;
    let chains = client
        .option_chain(SYMBOL, SecurityType::Stock, stock_id)
        .subscribe()
        .await
        .expect("option_chain failed")
        .collect_to_end_within(Duration::from_secs(30))
        .await
        .expect("option_chain did not end");
    let chain = chains.iter().find(|c| c.exchange == "SMART").expect("no SMART option chain");

    let cutoff: ExpirationDate = yyyymmdd_from_now(14).parse().expect("cutoff date");
    let mut later: Vec<ExpirationDate> = chain.expirations.iter().filter_map(Expiry::date).filter(|d| *d >= cutoff).collect();
    later.sort();
    assert!(later.len() >= 2, "need two expirations after {cutoff}: {later:?}");
    (stock_id, later[0].to_string(), later[1].to_string())
}

/// Five consecutive strikes listed in every map, around the middle, and a limit price inside
/// the max payoff of every option-only preset, so TWS's riskless / guaranteed-to-lose check (201) passes.
fn strike_window(maps: &[&BTreeMap<StrikeCents, i32>]) -> (Vec<StrikeCents>, f64) {
    let common: Vec<StrikeCents> = maps[0].keys().filter(|k| maps.iter().all(|m| m.contains_key(k))).copied().collect();
    assert!(common.len() >= 5, "too few shared strikes: {common:?}");
    let mid = common.len() / 2;
    let window = common[mid - 2..=mid + 2].to_vec();
    // Smallest gap: strike spacing can change inside the window (2.5 near the money, 5 further out).
    let width = dollars(window.windows(2).map(|w| w[1] - w[0]).min().unwrap());
    (window, (width / 2.0).min(1.0))
}

#[tokio::test]
async fn spread_presets_accepted_by_what_if() {
    let client_id = ClientId::get();
    let client = connect(&client_id).await;

    let (stock, near, far) = underlying_and_expiries(&client).await;
    let puts = strikes(&client, &near, OptionRight::Put).await;
    let calls = strikes(&client, &near, OptionRight::Call).await;
    let far_calls = strikes(&client, &far, OptionRight::Call).await;
    let (k, price) = strike_window(&[&calls, &puts, &far_calls]);

    const SHARES: i32 = 100;
    let spread = || Contract::spread(SYMBOL);
    let cases: Vec<(&str, SpreadBuilder<HasLegs>, f64)> = vec![
        ("vertical", spread().vertical(calls[&k[2]], calls[&k[3]]), price),
        ("long_calendar", spread().long_calendar(calls[&k[2]], far_calls[&k[2]]), price),
        // Buys the near leg, sells the dearer far one: a credit, bought at a negative price.
        ("short_calendar", spread().short_calendar(calls[&k[2]], far_calls[&k[2]]), -price),
        ("straddle", spread().straddle(calls[&k[2]], puts[&k[2]]), price),
        ("strangle", spread().strangle(calls[&k[3]], puts[&k[1]]), price),
        (
            "butterfly",
            spread().butterfly(ButterflyLegs {
                lower: calls[&k[1]],
                middle: calls[&k[2]],
                upper: calls[&k[3]],
            }),
            price / 2.0,
        ),
        // Opens for a credit: bought at a negative price.
        (
            "iron_condor",
            spread().iron_condor(IronCondorLegs {
                long_put: puts[&k[0]],
                short_put: puts[&k[1]],
                short_call: calls[&k[3]],
                long_call: calls[&k[4]],
            }),
            -price,
        ),
        // A price inside the payoff, put floor to call cap (× shares), wherever spot is. TWS
        // doesn't enforce that range for a stock + option combo (live: accepted outside it).
        (
            "collar",
            spread().collar(CollarLegs {
                stock,
                put: puts[&k[1]],
                call: calls[&k[3]],
                shares_per_contract: SHARES,
            }),
            f64::from(SHARES) * dollars(k[2]),
        ),
    ];

    let mut failures = Vec::new();
    for (name, builder, limit) in cases {
        let combo = builder.build().expect("spread build failed");
        match client.order(&combo).buy(1).limit(limit).analyze().await {
            Ok(state) => eprintln!("{name} @ {limit}: {:?}", state.status),
            Err(e) => failures.push(format!("{name} @ {limit}: {e}")),
        }
    }
    assert!(
        failures.is_empty(),
        "what-if rejected (expiries {near}/{far}, strikes {:?}):\n{}",
        k.iter().copied().map(dollars).collect::<Vec<_>>(),
        failures.join("\n")
    );
}

/// A vertical built by `Contract::spread_from_legs` from the leg contracts `contract_details`
/// returns, without naming the combo's symbol, currency or exchange.
#[tokio::test]
async fn spread_from_legs_accepted_by_what_if() {
    let client_id = ClientId::get();
    let client = connect(&client_id).await;

    let (_, near, _) = underlying_and_expiries(&client).await;
    let mut calls = client
        .contract_details(&Contract::option(SYMBOL, near.parse().expect("YYYYMMDD expiry"), 0.0, OptionRight::Call))
        .await
        .expect("option contract details failed");
    calls.retain(|d| d.contract.trading_class == SYMBOL);
    calls.sort_by(|a, b| a.contract.strike.total_cmp(&b.contract.strike));
    let mid = calls.len() / 2;
    let (long, short) = (&calls[mid].contract, &calls[mid + 1].contract);
    let price = ((short.strike - long.strike) / 2.0).min(1.0);

    let vertical = Contract::spread_from_legs([(long, 1), (short, -1)]).expect("spread_from_legs failed");
    let state = client
        .order(&vertical)
        .buy(1)
        .limit(price)
        .analyze()
        .await
        .unwrap_or_else(|e| panic!("what-if rejected {}/{} @ {price}: {e}", long.strike, short.strike));
    eprintln!("spread_from_legs vertical @ {price}: {:?}", state.status);
}
