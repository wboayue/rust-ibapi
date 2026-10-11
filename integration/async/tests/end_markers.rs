//! Diagnostic: does TWS send a stream's end marker only after the initial dump,
//! or again after later pushes? Subscribes to the five dump-then-delta streams,
//! optionally fills one ES contract to force pushes, and prints every frame's
//! arrival time plus a per-stream summary. Asserts only that each stream ended
//! its initial dump; the summary is the finding.
//!
//! ```text
//! END_MARKER_OBSERVE_SECS=420 END_MARKER_FILL=1 \
//!   cargo test -p ibapi-integration-async --test end_markers -- --ignored --nocapture
//! ```
//!
//! `END_MARKER_FILL=1` buys one front-month ES at 20s and sells it at 60s
//! (needs Globex open). Observe for at least two 3-minute account push cycles.

use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::Arc;

use futures::stream::{select_all, Stream};
use futures::StreamExt;
use ibapi::accounts::types::{AccountGroup, AccountId};
use ibapi::accounts::{AccountSummaryResult, AccountSummaryTags, AccountUpdate, AccountUpdateMulti, PositionUpdate, PositionUpdateMulti};
use ibapi::contracts::Contract;
use ibapi::orders::{Action, Order, OrderStatusKind, PlaceOrder};
use ibapi::subscriptions::SubscriptionItem;
use ibapi::{Client, Error};
use ibapi_test::{rate_limit, require_globex_open, yyyymmdd_from_now, ClientId, GATEWAY};
use serial_test::serial;
use tokio::time::{sleep, timeout, Duration, Instant};

#[derive(Debug)]
enum Kind {
    Row,
    End,
    Notice(i32),
    Error(String),
    Closed,
}

type Events = Pin<Box<dyn Stream<Item = (&'static str, Kind)> + Send>>;

fn events<S, T: 'static>(label: &'static str, stream: S, is_end: fn(&T) -> bool) -> Events
where
    S: Stream<Item = Result<SubscriptionItem<T>, Error>> + Send + 'static,
{
    let kinds = stream.map(move |item| match item {
        Ok(SubscriptionItem::Data(value)) if is_end(&value) => Kind::End,
        Ok(SubscriptionItem::Data(_)) => Kind::Row,
        Ok(SubscriptionItem::Notice(notice)) => Kind::Notice(notice.code),
        Err(e) => Kind::Error(e.to_string()),
    });
    // A closed stream is a finding too: it means the marker ended the subscription.
    kinds
        .chain(futures::stream::once(async { Kind::Closed }))
        .map(move |kind| (label, kind))
        .boxed()
}

const INITIAL_WINDOW: Duration = Duration::from_secs(2);

#[derive(Default)]
struct Tally {
    rows: usize,
    ends: Vec<Duration>,
    rows_after_first_end: usize,
    last_row: Option<Duration>,
    /// An end marker that arrived after at least one pushed row. Rows within
    /// [`INITIAL_WINDOW`] of the first end are not pushes: `account_summary`
    /// repeats its `$LEDGER` block, with a second end, right after the first.
    end_after_push: bool,
    pushed_since_end: bool,
    closed: bool,
    notices: Vec<i32>,
    errors: Vec<String>,
}

impl Tally {
    fn record(&mut self, at: Duration, kind: &Kind) {
        match kind {
            Kind::Row => {
                self.rows += 1;
                self.last_row = Some(at);
                if let Some(first_end) = self.ends.first() {
                    self.rows_after_first_end += 1;
                    if at.saturating_sub(*first_end) > INITIAL_WINDOW {
                        self.pushed_since_end = true;
                    }
                }
            }
            Kind::End => {
                if self.pushed_since_end {
                    self.end_after_push = true;
                }
                self.pushed_since_end = false;
                self.ends.push(at);
            }
            Kind::Closed => self.closed = true,
            Kind::Notice(code) => self.notices.push(*code),
            Kind::Error(e) => self.errors.push(e.clone()),
        }
    }
}

fn env_secs(name: &str, default: u64) -> Duration {
    Duration::from_secs(std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default))
}

fn market_order(action: Action) -> Order {
    Order {
        action,
        total_quantity: 1.0,
        order_type: "MKT".to_string(),
        ..Default::default()
    }
}

async fn front_month_es(client: &Client) -> Contract {
    let query = Contract::futures("ES").on_exchange("CME").any_month().build();
    rate_limit();
    let details = client.contract_details(&query).await.expect("contract_details failed");
    let cutoff = yyyymmdd_from_now(7);
    details
        .into_iter()
        .map(|d| d.contract)
        .filter(|c| c.last_trade_date_or_contract_month > cutoff)
        .min_by(|a, b| a.last_trade_date_or_contract_month.cmp(&b.last_trade_date_or_contract_month))
        .expect("no ES listing beyond the cutoff")
}

async fn trade_and_wait(client: &Client, contract: &Contract, action: Action) {
    rate_limit();
    let order_id = client.next_order_id();
    let mut sub = client
        .place_order(order_id, contract, &market_order(action))
        .await
        .expect("place_order failed");
    let deadline = Instant::now() + Duration::from_secs(15);
    while let Ok(Some(item)) = timeout(deadline.saturating_duration_since(Instant::now()), sub.next()).await {
        if let Ok(SubscriptionItem::Data(PlaceOrder::OrderStatus(status))) = item {
            if status.status == OrderStatusKind::Filled {
                eprintln!("[fill] {action:?} ES filled");
                return;
            }
        }
    }
    eprintln!("warning: ES {action:?} did not report Filled within 15s; check the paper account");
}

/// Buy at 20s, sell at 60s, so pushes after the first end are forced.
async fn round_trip(client: Arc<Client>, start: Instant) {
    let contract = front_month_es(&client).await;
    sleep(Duration::from_secs(20).saturating_sub(start.elapsed())).await;
    trade_and_wait(&client, &contract, Action::Buy).await;
    sleep(Duration::from_secs(60).saturating_sub(start.elapsed())).await;
    trade_and_wait(&client, &contract, Action::Sell).await;
}

#[tokio::test]
#[serial(account)]
#[ignore = "diagnostic: runs for minutes; see module docs"]
async fn end_markers_after_initial_dump() {
    let observe = env_secs("END_MARKER_OBSERVE_SECS", 420);
    let fill = std::env::var("END_MARKER_FILL").is_ok_and(|v| v == "1");
    if fill {
        require_globex_open();
    }

    let client_id = ClientId::get();
    rate_limit();
    let client = Arc::new(Client::connect(GATEWAY, client_id.id()).await.expect("connection failed"));
    rate_limit();
    let accounts = client.managed_accounts().await.expect("managed_accounts failed");
    let account = AccountId::from(accounts[0].as_str());
    eprintln!("account {account}, observing {}s, fill={fill}", observe.as_secs());

    let group = AccountGroup("All".to_string());
    rate_limit();
    let summary = client
        .account_summary(&group, AccountSummaryTags::ALL)
        .await
        .expect("account_summary failed");
    rate_limit();
    let updates = client.account_updates(&account).await.expect("account_updates failed");
    rate_limit();
    let updates_multi = client
        .account_updates_multi(Some(&account), None)
        .await
        .expect("account_updates_multi failed");
    rate_limit();
    let positions = client.positions().await.expect("positions failed");
    rate_limit();
    let positions_multi = client.positions_multi(Some(&account), None).await.expect("positions_multi failed");

    let mut merged = select_all(vec![
        events("account_summary", summary, |v| matches!(v, AccountSummaryResult::End)),
        events("account_updates", updates, |v| matches!(v, AccountUpdate::End)),
        events("account_updates_multi", updates_multi, |v| matches!(v, AccountUpdateMulti::End)),
        events("positions", positions, |v| matches!(v, PositionUpdate::PositionEnd)),
        events("positions_multi", positions_multi, |v| matches!(v, PositionUpdateMulti::PositionEnd)),
    ]);

    let start = Instant::now();
    let trader = fill.then(|| tokio::spawn(round_trip(client.clone(), start)));

    let mut tallies: BTreeMap<&'static str, Tally> = BTreeMap::new();
    let deadline = start + observe;
    while let Ok(Some((label, kind))) = timeout(deadline.saturating_duration_since(Instant::now()), merged.next()).await {
        let at = start.elapsed();
        eprintln!("{:>9.3}s  {label:<22} {kind:?}", at.as_secs_f64());
        tallies.entry(label).or_default().record(at, &kind);
    }

    if let Some(trader) = trader {
        trader.await.expect("round trip panicked");
    }

    eprintln!(
        "\n{:<22} {:>5} {:>8} {:>10} {:>9} {:>11} {:>7}  end times",
        "stream", "rows", "ends", "after_end", "last_row", "end_re_sent", "closed"
    );
    for (label, t) in &tallies {
        let ends: Vec<String> = t.ends.iter().map(|d| format!("{:.3}s", d.as_secs_f64())).collect();
        let last_row = t.last_row.map_or("-".to_string(), |d| format!("{:.1}s", d.as_secs_f64()));
        eprintln!(
            "{label:<22} {:>5} {:>8} {:>10} {:>9} {:>11} {:>7}  {}",
            t.rows,
            t.ends.len(),
            t.rows_after_first_end,
            last_row,
            t.end_after_push,
            t.closed,
            ends.join(", ")
        );
    }
    for (label, t) in &tallies {
        if !t.notices.is_empty() || !t.errors.is_empty() {
            eprintln!("{label}: notices {:?}, errors {:?}", t.notices, t.errors);
        }
    }

    let missing: Vec<_> = [
        "account_summary",
        "account_updates",
        "account_updates_multi",
        "positions",
        "positions_multi",
    ]
    .into_iter()
    .filter(|label| tallies.get(label).is_none_or(|t| t.ends.is_empty()))
    .collect();
    assert!(missing.is_empty(), "no end marker for the initial dump: {missing:?}");
}

/// Which tags make `account_summary` send more than one end marker: the
/// regular tags alone, `$LEDGER:ALL` alone, then both. Prints rows per
/// account and the end-marker positions for each.
#[tokio::test]
#[serial(account)]
#[ignore = "diagnostic: see module docs"]
async fn account_summary_end_markers_by_tag_set() {
    let group = AccountGroup("All".to_string());

    let regular: Vec<&str> = AccountSummaryTags::ALL
        .iter()
        .copied()
        .filter(|t| *t != AccountSummaryTags::LEDGER_ALL)
        .collect();
    let cases: [(&str, &[&str]); 3] = [
        ("both", AccountSummaryTags::ALL),
        ("regular", &regular),
        ("ledger_all", &[AccountSummaryTags::LEDGER_ALL]),
    ];

    for (label, tags) in cases {
        // A fresh connection per case: TWS kept a cancelled request's slot on the
        // connection, so a third request on one client got 322.
        let client_id = ClientId::get();
        rate_limit();
        let client = Client::connect(GATEWAY, client_id.id()).await.expect("connection failed");
        rate_limit();
        let mut sub = client.account_summary(&group, tags).await.expect("account_summary failed");
        // One line per run of rows from the same account, and one per end marker.
        let mut runs: Vec<String> = Vec::new();
        let mut current: Option<(String, usize)> = None;
        let deadline = Instant::now() + Duration::from_secs(5);
        while let Ok(Some(item)) = timeout(deadline.saturating_duration_since(Instant::now()), sub.next()).await {
            match item {
                Ok(SubscriptionItem::Data(AccountSummaryResult::Summary(row))) => match &mut current {
                    Some((account, n)) if *account == row.account => *n += 1,
                    _ => {
                        if let Some((account, n)) = current.take() {
                            runs.push(format!("{n} {account}"));
                        }
                        current = Some((row.account, 1));
                    }
                },
                Ok(SubscriptionItem::Data(AccountSummaryResult::End)) => {
                    if let Some((account, n)) = current.take() {
                        runs.push(format!("{n} {account}"));
                    }
                    runs.push("End".to_string());
                }
                Ok(SubscriptionItem::Notice(notice)) => runs.push(format!("notice {}", notice.code)),
                Err(e) => runs.push(format!("error {e}")),
            }
        }
        if let Some((account, n)) = current.take() {
            runs.push(format!("{n} {account}"));
        }
        sub.cancel().await;
        eprintln!("{label:<11} {}", runs.join(", "));
    }
}
