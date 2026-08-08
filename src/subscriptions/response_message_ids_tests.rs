//! Cross-check of `RESPONSE_MESSAGE_IDS` against the `decode` match arms, for
//! every production decoder — both [`StreamDecoder`] and
//! [`TickDecoder`](crate::market_data::historical::TickDecoder).
//!
//! Since #732 the const is the sole skip filter: all three drivers discard a
//! frame whose type is absent from it before `decode` is ever called. That makes
//! the two lists a contract, and it can fail in either direction:
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
//!
//! That reading is what a decoder's backstop buys — the `_ =>` arm on a
//! multi-type `decode`, or `expect_type` on one that consumes a single type.
//! Without it every discriminant reads as handled, and the probe is blind. The
//! three `TickDecoder` impls had no backstop, which is why gating them meant
//! adding one first.

use std::collections::BTreeSet;

use super::{DecoderContext, StreamDecoder};
use crate::errors::Error;
use crate::market_data::historical::TickDecoder;
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

/// One decoder's declared list against its `decode` arms.
///
/// `decode` arrives as a closure because the two traits spell it differently —
/// `StreamDecoder` takes a `DecoderContext` and `&mut ResponseMessage`,
/// `TickDecoder` takes `&ResponseMessage` and returns a batch. Erasing that
/// difference here keeps the failure taxonomy below in one copy; the alternative
/// was a second `check` that drifts from this one, which is the shape of defect
/// this whole file exists to catch.
fn check_decoder(
    decoder: &str,
    declared_ids: &[IncomingMessages],
    decode: impl Fn(&mut ResponseMessage) -> Result<(), Error>,
    failures: &mut Vec<String>,
) {
    for kind in all_message_types() {
        let declared = declared_ids.contains(&kind);

        if declared && DISPATCHER_INTERCEPTED.contains(&kind) {
            failures.push(format!(
                "{decoder} declares {kind:?}, which the dispatcher intercepts before routing — \
                 it never arrives as a `RoutedItem::Response`, so `decode` cannot see it"
            ));
            continue;
        }

        let handled = !matches!(decode(&mut probe(kind)), Err(Error::UnexpectedResponse(_)));

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

fn check_stream<D: StreamDecoder<D>>(failures: &mut Vec<String>) {
    let context = DecoderContext::default();
    check_decoder(
        std::any::type_name::<D>(),
        D::RESPONSE_MESSAGE_IDS,
        |message| <D as StreamDecoder<D>>::decode(&context, message).map(|_| ()),
        failures,
    );
}

/// The tick driver (`market_data/historical/common/tick.rs::classify`) filters on
/// the same const through the same `is_undeclared` helper, so it is subject to
/// the same contract. It went ungated when #738 gave `TickDecoder` the const,
/// because the roster collector matched `impl StreamDecoder<` alone.
fn check_tick<T: TickDecoder<T>>(failures: &mut Vec<String>) {
    check_decoder(
        std::any::type_name::<T>(),
        T::RESPONSE_MESSAGE_IDS,
        |message| <T as TickDecoder<T>>::decode(message).map(|_| ()),
        failures,
    );
}

/// The roster. One line per production `impl StreamDecoder` / `impl TickDecoder`;
/// kept honest by [`test_decoder_roster_is_complete`].
fn check_all(failures: &mut Vec<String>) {
    use crate::accounts::{AccountSummaryResult, AccountUpdate, AccountUpdateMulti, PnL, PnLSingle, PositionUpdate, PositionUpdateMulti};
    use crate::contracts::{OptionChain, OptionComputation};
    use crate::display_groups::DisplayGroupUpdate;
    use crate::market_data::historical::{HistoricalBarUpdate, TickBidAsk, TickLast, TickMidpoint};
    use crate::market_data::realtime::{Bar, BidAsk, MarketDepths, MidPoint, TickTypes, Trade};
    use crate::news::{NewsArticle, NewsBulletin};
    use crate::orders::{CancelOrder, Executions, ExerciseOptions, OrderUpdate, Orders, PlaceOrder};
    use crate::scanner::ScannerData;
    use crate::wsh::{WshEventData, WshMetadata};

    check_stream::<AccountSummaryResult>(failures);
    check_stream::<AccountUpdate>(failures);
    check_stream::<AccountUpdateMulti>(failures);
    check_stream::<PnL>(failures);
    check_stream::<PnLSingle>(failures);
    check_stream::<PositionUpdate>(failures);
    check_stream::<PositionUpdateMulti>(failures);
    check_stream::<OptionChain>(failures);
    check_stream::<OptionComputation>(failures);
    check_stream::<DisplayGroupUpdate>(failures);
    check_stream::<HistoricalBarUpdate>(failures);
    check_stream::<Bar>(failures);
    check_stream::<BidAsk>(failures);
    check_stream::<MarketDepths>(failures);
    check_stream::<MidPoint>(failures);
    check_stream::<TickTypes>(failures);
    check_stream::<Trade>(failures);
    check_stream::<NewsArticle>(failures);
    check_stream::<NewsBulletin>(failures);
    check_stream::<CancelOrder>(failures);
    check_stream::<Executions>(failures);
    check_stream::<ExerciseOptions>(failures);
    check_stream::<OrderUpdate>(failures);
    check_stream::<Orders>(failures);
    check_stream::<PlaceOrder>(failures);
    check_stream::<Vec<ScannerData>>(failures);
    check_stream::<WshEventData>(failures);
    check_stream::<WshMetadata>(failures);

    check_tick::<TickBidAsk>(failures);
    check_tick::<TickLast>(failures);
    check_tick::<TickMidpoint>(failures);
}

/// Number of `check_stream::<_>` / `check_tick::<_>` calls in [`check_all`],
/// per trait. Compared against the tree by [`test_decoder_roster_is_complete`].
const STREAM_ROSTER_LEN: usize = 28;
const TICK_ROSTER_LEN: usize = 3;

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
///
/// Counted per trait rather than in total, so a `TickDecoder` added while a
/// `StreamDecoder` is deleted cannot net out to zero.
#[test]
fn test_decoder_roster_is_complete() {
    assert_impl_count("impl StreamDecoder<", STREAM_ROSTER_LEN, "`check_stream` and bump `STREAM_ROSTER_LEN`");
    assert_impl_count("impl TickDecoder<", TICK_ROSTER_LEN, "`check_tick` and bump `TICK_ROSTER_LEN`");
}

/// `remedy` arrives pre-composed rather than as an `adder` / `const_name` pair,
/// so the signature stays at three and does not put three same-typed `&str` in a
/// row — see `docs/rules/style/param-budget.md`.
fn assert_impl_count(header: &str, expected: usize, remedy: &str) {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut impls = Vec::new();
    collect_impls(&src, header, &mut impls);
    impls.sort();

    assert_eq!(
        impls.len(),
        expected,
        "src/ has {} `{header}` blocks but the roster in this file lists {expected}. \
         Add the new decoder to `check_all` with {remedy}.\nFound:\n  {}",
        impls.len(),
        impls.join("\n  ")
    );
}

/// Production `impl <Trait><..> for ..` headers under `dir`. Test-only decoders
/// live in `*_tests.rs` / `tests.rs` and are skipped — they exist to exercise the
/// drivers, not to decode a wire message.
///
/// Matching on the header's own prefix excludes generic bounds, which read
/// `impl<T: TickDecoder<T>> ..` and are not impls of the trait.
fn collect_impls(dir: &std::path::Path, header: &str, found: &mut Vec<String>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()));

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_impls(&path, header, found);
            continue;
        }

        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if !name.ends_with(".rs") || name == "tests.rs" || name.ends_with("_tests.rs") {
            continue;
        }

        let contents = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        for line in contents.lines() {
            if line.trim_start().starts_with(header) {
                found.push(format!("{}: {}", path.display(), line.trim()));
            }
        }
    }
}
