//! Diagnostic capture for `plans/retire-response-message-public-surface.md` PR 1.
//!
//! Records every `StartupMessage` the handshake delivers and prints an
//! aggregate table on exit. The interesting set is `StartupMessage::Other(_)` —
//! the message kinds that fall through to the public `ResponseMessage` payload
//! we want to retire. The plan's PR 2 adds typed variants for them.
//!
//! How to run (paper account, port 4002 default — adjust for live):
//!
//! ```bash
//! # Plain connect — covers initial handshake only.
//! cargo run --example async_startup_capture
//!
//! # Master Client ID needed for unsolicited CommissionsReport / OpenOrder
//! # replays (see TWS docs §master_client). Configure the gateway's Master
//! # Client ID to 100, restart it, then:
//! cargo run --example async_startup_capture
//! ```
//!
//! To cover the doc-comment's full hypothesised set (`ExecutionData`,
//! `CommissionsReport`, `CompletedOrder`), run the capture across these three
//! scenarios sequentially. The plan author should append the union of
//! observed `Other` kinds back into the audit table.
//!
//! 1. **Outstanding orders**: place a working LMT order off-market (e.g.
//!    AAPL @ $1), leave it open, reconnect.
//! 2. **Recent fills**: place a marketable order to fill (paper account
//!    only — be sure it's a non-tradeable security or off-hours), reconnect.
//! 3. **Completed orders history**: after fills, reconnect to pull the
//!    completed-orders replay.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use ibapi::{Client, IncomingMessages, StartupMessage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let observed: Arc<Mutex<Vec<(IncomingMessages, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let observed_cb = observed.clone();

    println!("Connecting to IB Gateway and capturing handshake messages...");

    // Pass the Master Client ID configured in the gateway as the first arg
    // (defaults to 0). Master Client ID is the one TWS spec'd to replay open
    // orders + commission reports unsolicited at handshake (see TWS docs
    // §master_client).
    let client_id: i32 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);

    println!("Using client_id={client_id} (must match gateway Master Client ID for unsolicited handshake replay)");

    let client = Client::builder()
        .address("127.0.0.1:4002")
        .client_id(client_id)
        .startup_callback(move |msg| {
            let kind = msg.message_type();
            let detail = match &msg {
                StartupMessage::OpenOrder(o) => format!("order_id={}", o.order_id),
                StartupMessage::OrderStatus(s) => format!("order_id={} status={}", s.order_id, s.status),
                StartupMessage::Execution(e) => format!("order_id={} exec_id={}", e.execution.order_id, e.execution.execution_id),
                StartupMessage::CommissionReport(c) => format!("exec_id={} commission={}", c.execution_id, c.commission),
                StartupMessage::CompletedOrder(o) => format!("perm_id={}", o.order.perm_id),
                StartupMessage::Other(rm) => format!("request_id={:?}", rm.request_id()),
                _ => String::new(),
            };
            observed_cb.lock().unwrap().push((kind, detail));
        })
        .connect()
        .await?;

    println!("Connected. server_version={}", client.server_version());

    // Give TWS a beat to fire any post-handshake unsolicited frames; everything
    // routed through the startup callback should already have arrived by the
    // time `connect()` returned, but keep the connection open briefly so log
    // observers can confirm nothing dribbles in late.
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let captured = observed.lock().unwrap();

    // Aggregate by IncomingMessages kind.
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for (kind, _) in captured.iter() {
        *counts.entry(format!("{kind:?}")).or_default() += 1;
    }

    println!("\n=== Handshake capture summary ===");
    println!("Total messages: {}", captured.len());
    if counts.is_empty() {
        println!("(no unsolicited handshake messages observed)");
    } else {
        for (kind, count) in &counts {
            println!("  {count:>4} × {kind}");
        }
    }

    println!("\n=== Per-message detail ===");
    for (kind, detail) in captured.iter() {
        if detail.is_empty() {
            println!("  {kind:?}");
        } else {
            println!("  {kind:?}  {detail}");
        }
    }

    drop(client);
    Ok(())
}
