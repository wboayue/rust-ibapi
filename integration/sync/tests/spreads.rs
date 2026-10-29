//! Placement checks for `SpreadBuilder` combos: each preset is sent to TWS as a what-if order,
//! which TWS validates without placing. Building a combo is unit-tested; only TWS can say it is
//! placeable (a combo without a symbol builds but draws 321, #1128).

use std::collections::BTreeMap;
use std::time::Duration;

use ibapi::client::blocking::Client;
use ibapi::contracts::{ButterflyLegs, Contract, IronCondorLegs, OptionRight, SecurityType, SpreadBuilder};
use ibapi_integration_sync::connect;
use ibapi_test::{yyyymmdd_from_now, ClientId};

const SYMBOL: &str = "AAPL";

/// Strike in integer cents, so it can be a map key.
type StrikeCents = i64;

/// Strike → contract id for one expiry and right. Strike 0 asks TWS for every strike.
///
/// Assumes one trading class per strike (true for AAPL); a second class would silently replace
/// the first in the map.
fn strikes(client: &Client, expiry: &str, right: OptionRight) -> BTreeMap<StrikeCents, i32> {
    let details = client
        .contract_details(&Contract::option(SYMBOL, expiry, 0.0, right))
        .expect("option contract details failed");
    details
        .iter()
        .map(|d| ((d.contract.strike * 100.0).round() as i64, d.contract.contract_id))
        .collect()
}

/// The two nearest expirations at least two weeks out, so neither expires mid-run.
fn expiries(client: &Client) -> (String, String) {
    let stock = client
        .contract_details(&Contract::stock(SYMBOL).build())
        .expect("stock contract details failed");
    let chains = client
        .option_chain(SYMBOL, SecurityType::Stock, stock[0].contract.contract_id)
        .subscribe()
        .expect("option_chain failed")
        .collect_to_end_within(Duration::from_secs(30))
        .expect("option_chain did not end");
    let chain = chains.iter().find(|c| c.exchange == "SMART").expect("no SMART option chain");

    let cutoff = yyyymmdd_from_now(14);
    let mut later: Vec<&String> = chain.expirations.iter().filter(|e| **e >= cutoff).collect();
    later.sort();
    assert!(later.len() >= 2, "need two expirations after {cutoff}: {later:?}");
    (later[0].clone(), later[1].clone())
}

/// Five consecutive strikes listed in every map, around the middle, and a limit price inside
/// every preset's max payoff, so TWS's riskless / guaranteed-to-lose check (201) passes.
fn strike_window(maps: &[&BTreeMap<StrikeCents, i32>]) -> (Vec<StrikeCents>, f64) {
    let common: Vec<StrikeCents> = maps[0].keys().filter(|k| maps.iter().all(|m| m.contains_key(k))).copied().collect();
    assert!(common.len() >= 5, "too few shared strikes: {common:?}");
    let mid = common.len() / 2;
    let window = common[mid - 2..=mid + 2].to_vec();
    // Smallest gap: strike spacing can change inside the window (2.5 near the money, 5 further out).
    let width = window.windows(2).map(|w| w[1] - w[0]).min().unwrap() as f64 / 100.0;
    (window, (width / 2.0).min(1.0))
}

#[test]
fn spread_presets_accepted_by_what_if() {
    let client_id = ClientId::get();
    let client = connect(&client_id);

    let (near, far) = expiries(&client);
    let puts = strikes(&client, &near, OptionRight::Put);
    let calls = strikes(&client, &near, OptionRight::Call);
    let far_calls = strikes(&client, &far, OptionRight::Call);
    let (k, price) = strike_window(&[&calls, &puts, &far_calls]);

    let spread = || Contract::spread().symbol(SYMBOL);
    let cases: Vec<(&str, SpreadBuilder, f64)> = vec![
        ("vertical", spread().vertical(calls[&k[2]], calls[&k[3]]), price),
        ("long_calendar", spread().long_calendar(calls[&k[2]], far_calls[&k[2]]), price),
        // Buys the near leg, sells the dearer far one: a credit, bought at a negative price.
        ("short_calendar", spread().short_calendar(calls[&k[2]], far_calls[&k[2]]), -price),
        ("straddle", spread().straddle(calls[&k[2]], puts[&k[2]]), price),
        ("strangle", spread().strangle(calls[&k[3]], puts[&k[1]]), price),
        (
            "butterfly",
            spread().butterfly(ButterflyLegs {
                lower: calls[&k[1]].into(),
                middle: calls[&k[2]].into(),
                upper: calls[&k[3]].into(),
            }),
            price / 2.0,
        ),
        // Opens for a credit: bought at a negative price.
        (
            "iron_condor",
            spread().iron_condor(IronCondorLegs {
                long_put: puts[&k[0]].into(),
                short_put: puts[&k[1]].into(),
                short_call: calls[&k[3]].into(),
                long_call: calls[&k[4]].into(),
            }),
            -price,
        ),
    ];

    let mut failures = Vec::new();
    for (name, builder, limit) in cases {
        let combo = builder.build().expect("spread build failed");
        match client.order(&combo).buy(1).limit(limit).analyze() {
            Ok(state) => eprintln!("{name} @ {limit}: {:?}", state.status),
            Err(e) => failures.push(format!("{name} @ {limit}: {e}")),
        }
    }
    assert!(
        failures.is_empty(),
        "what-if rejected (expiries {near}/{far}, strikes {:?}):\n{}",
        k.iter().map(|c| *c as f64 / 100.0).collect::<Vec<_>>(),
        failures.join("\n")
    );
}
