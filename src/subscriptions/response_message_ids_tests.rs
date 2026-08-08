//! Cross-check of [`StreamDecoder::RESPONSE_MESSAGE_IDS`] against the `decode`
//! match arms, for every production decoder.
//!
//! Since #732 the const is the sole skip filter: both drivers discard a frame
//! whose type is absent from it before `decode` is ever called. That makes the
//! two lists a contract, and it can fail in either direction:
//!
//! - **Declared but unhandled** — the const names a type `decode` has no arm
//!   for. Loud already: the `_ =>` backstop returns `UnexpectedResponse` and
//!   the subscription fails.
//! - **Handled but undeclared** — `decode` has an arm the const omits. Silent,
//!   and the worse direction: the driver filters the frame out, the arm is
//!   unreachable, and the data disappears with nothing to observe. This is the
//!   direction #732 left open and this test closes.
//!
//! The probe is a minimal *text*-framed message carrying only the discriminant.
//! That is what makes the two cases mechanically distinguishable without a
//! per-type fixture: no arm falls through to `Error::unexpected_response`,
//! while every real arm either returns `Ok`/`EndOfStream` without touching the
//! payload or reaches `require_proto()` and returns `Error::UnexpectedWireFormat`
//! (#731). So `UnexpectedResponse` means "no arm", and anything else means
//! "arm exists".

use std::collections::BTreeSet;

use super::{DecoderContext, StreamDecoder};
use crate::errors::Error;
use crate::messages::{IncomingMessages, ResponseMessage};

/// Discriminants to probe. Assigned ids run `-2..=111`; the range is a
/// deliberate superset so a newly assigned id is covered without editing this
/// file. Unassigned ids all map to `NotValid`, which is deduped to one probe.
const DISCRIMINANT_SCAN: std::ops::RangeInclusive<i32> = -2..=255;

fn all_message_types() -> Vec<IncomingMessages> {
    let mut seen = BTreeSet::new();
    DISCRIMINANT_SCAN
        .map(IncomingMessages::from)
        .filter(|kind| seen.insert(*kind as i32))
        .collect()
}

/// A frame carrying the discriminant and nothing else. Text-framed on purpose —
/// see the module doc.
fn probe(kind: IncomingMessages) -> ResponseMessage {
    ResponseMessage::from(&format!("{}\0", kind as i32))
}

/// Message types [`determine_routing`](crate::transport::routing) classifies
/// before any allow-list is consulted: `Error` becomes `RoutedItem::Error` or
/// `RoutedItem::Notice`, `Shutdown` ends the dispatcher loop. Neither ever
/// arrives as a `RoutedItem::Response`, so no `decode` can see one and
/// declaring either is a claim the dispatcher contradicts.
///
/// Sixteen decoders declared `Error` before this check existed, and
/// `routable_to_request_id_subscription` had to exempt `Error` to stop those
/// declarations tripping the routing guard — const declares, guard exempts,
/// circular. The exemption is gone; this keeps the declarations gone with it.
const DISPATCHER_INTERCEPTED: &[IncomingMessages] = &[IncomingMessages::Error, IncomingMessages::Shutdown];

fn check<D: StreamDecoder<D>>(failures: &mut Vec<String>) {
    let decoder = std::any::type_name::<D>();
    let context = DecoderContext::default();

    for kind in all_message_types() {
        let declared = D::RESPONSE_MESSAGE_IDS.contains(&kind);

        if declared && DISPATCHER_INTERCEPTED.contains(&kind) {
            failures.push(format!(
                "{decoder} declares {kind:?}, which the dispatcher intercepts before routing — \
                 it never arrives as a `RoutedItem::Response`, so `decode` cannot see it"
            ));
            continue;
        }

        let handled = !matches!(
            <D as StreamDecoder<D>>::decode(&context, &mut probe(kind)),
            Err(Error::UnexpectedResponse(_))
        );

        match (declared, handled) {
            (true, false) => failures.push(format!(
                "{decoder} declares {kind:?} in RESPONSE_MESSAGE_IDS but `decode` has no arm for it — \
                 the declaration is dead and the frame fails the subscription"
            )),
            (false, true) => failures.push(format!(
                "{decoder} has a `decode` arm for {kind:?} but does not declare it in RESPONSE_MESSAGE_IDS — \
                 the driver drops the frame before `decode`, so the arm is unreachable and the data is lost silently"
            )),
            _ => {}
        }
    }
}

/// The roster. One line per production `impl StreamDecoder`; kept honest by
/// [`test_decoder_roster_is_complete`].
fn check_all(failures: &mut Vec<String>) {
    use crate::accounts::{AccountSummaryResult, AccountUpdate, AccountUpdateMulti, PnL, PnLSingle, PositionUpdate, PositionUpdateMulti};
    use crate::contracts::{OptionChain, OptionComputation};
    use crate::display_groups::DisplayGroupUpdate;
    use crate::market_data::historical::HistoricalBarUpdate;
    use crate::market_data::realtime::{Bar, BidAsk, MarketDepths, MidPoint, TickTypes, Trade};
    use crate::news::{NewsArticle, NewsBulletin};
    use crate::orders::{CancelOrder, Executions, ExerciseOptions, OrderUpdate, Orders, PlaceOrder};
    use crate::scanner::ScannerData;
    use crate::wsh::{WshEventData, WshMetadata};

    check::<AccountSummaryResult>(failures);
    check::<AccountUpdate>(failures);
    check::<AccountUpdateMulti>(failures);
    check::<PnL>(failures);
    check::<PnLSingle>(failures);
    check::<PositionUpdate>(failures);
    check::<PositionUpdateMulti>(failures);
    check::<OptionChain>(failures);
    check::<OptionComputation>(failures);
    check::<DisplayGroupUpdate>(failures);
    check::<HistoricalBarUpdate>(failures);
    check::<Bar>(failures);
    check::<BidAsk>(failures);
    check::<MarketDepths>(failures);
    check::<MidPoint>(failures);
    check::<TickTypes>(failures);
    check::<Trade>(failures);
    check::<NewsArticle>(failures);
    check::<NewsBulletin>(failures);
    check::<CancelOrder>(failures);
    check::<Executions>(failures);
    check::<ExerciseOptions>(failures);
    check::<OrderUpdate>(failures);
    check::<Orders>(failures);
    check::<PlaceOrder>(failures);
    check::<Vec<ScannerData>>(failures);
    check::<WshEventData>(failures);
    check::<WshMetadata>(failures);
}

/// Number of `check::<_>` calls in [`check_all`]. Compared against the tree by
/// [`test_decoder_roster_is_complete`].
const ROSTER_LEN: usize = 28;

#[test]
fn test_response_message_ids_match_decode_arms() {
    let mut failures = Vec::new();
    check_all(&mut failures);

    assert!(
        failures.is_empty(),
        "RESPONSE_MESSAGE_IDS and `decode` disagree:\n  {}",
        failures.join("\n  ")
    );
}

/// A hand-listed roster rots the moment someone adds a decoder, and the rot is
/// silent — the new decoder is simply never checked. Counting the impls in the
/// tree turns that into a failing test.
#[test]
fn test_decoder_roster_is_complete() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut impls = Vec::new();
    collect_stream_decoder_impls(&src, &mut impls);
    impls.sort();

    assert_eq!(
        impls.len(),
        ROSTER_LEN,
        "src/ has {} `impl StreamDecoder` blocks but the roster in this file lists {ROSTER_LEN}. \
         Add the new decoder to `check_all` and bump `ROSTER_LEN`.\nFound:\n  {}",
        impls.len(),
        impls.join("\n  ")
    );
}

/// Production `impl StreamDecoder<..> for ..` headers under `dir`. Test-only
/// decoders live in `*_tests.rs` / `tests.rs` and are skipped — they exist to
/// exercise the drivers, not to decode a wire message.
fn collect_stream_decoder_impls(dir: &std::path::Path, found: &mut Vec<String>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()));

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_stream_decoder_impls(&path, found);
            continue;
        }

        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if !name.ends_with(".rs") || name == "tests.rs" || name.ends_with("_tests.rs") {
            continue;
        }

        let contents = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        for line in contents.lines() {
            if line.trim_start().starts_with("impl StreamDecoder<") {
                found.push(format!("{}: {}", path.display(), line.trim()));
            }
        }
    }
}
