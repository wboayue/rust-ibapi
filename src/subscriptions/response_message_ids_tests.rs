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
//! "arm exists" — which only holds while every `decode` has a backstop. One that
//! dives straight into `require_proto()` reads as handling everything.

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
/// `StreamDecoder` also takes a `DecoderContext`, and `TickDecoder` returns a
/// batch. Erasing that difference here keeps the failure taxonomy below in one
/// copy; the alternative was a second `check` that drifts from this one, which
/// is the shape of defect this whole file exists to catch.
fn check_decoder(decoder: &str, declared_ids: &[IncomingMessages], decode: impl Fn(&ResponseMessage) -> Result<(), Error>) -> Vec<String> {
    let mut failures = Vec::new();

    for kind in all_message_types() {
        let declared = declared_ids.contains(&kind);

        if declared && DISPATCHER_INTERCEPTED.contains(&kind) {
            failures.push(format!(
                "{decoder} declares {kind:?}, which the dispatcher intercepts before routing — \
                 it never arrives as a `RoutedItem::Response`, so `decode` cannot see it"
            ));
            continue;
        }

        let handled = !matches!(decode(&probe(kind)), Err(Error::UnexpectedResponse(_)));

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

    failures
}

/// What [`check_all`] accumulates: the failures, and how many decoders of each
/// trait it actually visited.
///
/// The counts are tallied by the `check_*` calls themselves rather than declared
/// as constants. A hand-typed count is a second statement of what the roster
/// already says, and it can agree with the tree while disagreeing with the
/// roster — add a decoder, bump the constant, forget the `check_stream::<Foo>`
/// line, and the tree count matches while `Foo` is never probed. Counting the
/// calls makes the roster itself the thing under test.
#[derive(Default)]
struct Roster {
    failures: Vec<String>,
    stream: usize,
    tick: usize,
}

fn check_stream<D: StreamDecoder<D>>(roster: &mut Roster) {
    let context = DecoderContext::default();
    roster.stream += 1;
    roster
        .failures
        .extend(check_decoder(std::any::type_name::<D>(), D::RESPONSE_MESSAGE_IDS, |message| {
            <D as StreamDecoder<D>>::decode(&context, message).map(|_| ())
        }));
}

/// The tick driver (`market_data/historical/common/tick.rs::classify`) filters on
/// the same const through the same `is_undeclared` helper, so it is subject to
/// the same contract.
fn check_tick<T: TickDecoder<T>>(roster: &mut Roster) {
    roster.tick += 1;
    roster
        .failures
        .extend(check_decoder(std::any::type_name::<T>(), T::RESPONSE_MESSAGE_IDS, |message| {
            <T as TickDecoder<T>>::decode(message).map(|_| ())
        }));
}

/// The roster. One line per production `impl StreamDecoder` / `impl TickDecoder`;
/// kept honest by [`test_decoder_roster_is_complete`].
fn check_all() -> Roster {
    use crate::accounts::{AccountSummaryResult, AccountUpdate, AccountUpdateMulti, PnL, PnLSingle, PositionUpdate, PositionUpdateMulti};
    use crate::contracts::{OptionChain, OptionComputation};
    use crate::display_groups::DisplayGroupUpdate;
    use crate::market_data::historical::{HistoricalBarUpdate, TickBidAsk, TickLast, TickMidpoint};
    use crate::market_data::realtime::{Bar, BidAsk, MarketDepths, MidPoint, TickTypes, Trade};
    use crate::news::{NewsArticle, NewsBulletin};
    use crate::orders::{CancelOrder, Executions, ExerciseOptions, OrderUpdate, Orders, PlaceOrder};
    use crate::scanner::ScannerData;
    use crate::wsh::{WshEventData, WshMetadata};

    let mut roster = Roster::default();

    check_stream::<AccountSummaryResult>(&mut roster);
    check_stream::<AccountUpdate>(&mut roster);
    check_stream::<AccountUpdateMulti>(&mut roster);
    check_stream::<PnL>(&mut roster);
    check_stream::<PnLSingle>(&mut roster);
    check_stream::<PositionUpdate>(&mut roster);
    check_stream::<PositionUpdateMulti>(&mut roster);
    check_stream::<OptionChain>(&mut roster);
    check_stream::<OptionComputation>(&mut roster);
    check_stream::<DisplayGroupUpdate>(&mut roster);
    check_stream::<HistoricalBarUpdate>(&mut roster);
    check_stream::<Bar>(&mut roster);
    check_stream::<BidAsk>(&mut roster);
    check_stream::<MarketDepths>(&mut roster);
    check_stream::<MidPoint>(&mut roster);
    check_stream::<TickTypes>(&mut roster);
    check_stream::<Trade>(&mut roster);
    check_stream::<NewsArticle>(&mut roster);
    check_stream::<NewsBulletin>(&mut roster);
    check_stream::<CancelOrder>(&mut roster);
    check_stream::<Executions>(&mut roster);
    check_stream::<ExerciseOptions>(&mut roster);
    check_stream::<OrderUpdate>(&mut roster);
    check_stream::<Orders>(&mut roster);
    check_stream::<PlaceOrder>(&mut roster);
    check_stream::<Vec<ScannerData>>(&mut roster);
    check_stream::<WshEventData>(&mut roster);
    check_stream::<WshMetadata>(&mut roster);

    check_tick::<TickBidAsk>(&mut roster);
    check_tick::<TickLast>(&mut roster);
    check_tick::<TickMidpoint>(&mut roster);

    roster
}

#[test]
fn test_response_message_ids_match_decode_arms() {
    let roster = check_all();

    assert!(
        roster.failures.is_empty(),
        "RESPONSE_MESSAGE_IDS and `decode` disagree:\n  {}",
        roster.failures.join("\n  ")
    );
}

/// A hand-listed roster rots the moment someone adds a decoder, and the rot is
/// silent — the new decoder is simply never checked. Counting the impls in the
/// tree and comparing against what [`check_all`] actually visited turns that
/// into a failing test.
///
/// Counted per trait, so a `TickDecoder` added while a `StreamDecoder` is
/// deleted cannot net out to zero.
#[test]
fn test_decoder_roster_is_complete() {
    let roster = check_all();
    let found = collect_impls(&[STREAM_HEADER, TICK_HEADER]);

    assert_roster_covers(STREAM_HEADER, &found[0], roster.stream);
    assert_roster_covers(TICK_HEADER, &found[1], roster.tick);
}

const STREAM_HEADER: &str = "impl StreamDecoder<";
const TICK_HEADER: &str = "impl TickDecoder<";

fn assert_roster_covers(header: &str, found: &[String], checked: usize) {
    assert_eq!(
        found.len(),
        checked,
        "src/ has {} `{header}` blocks but `check_all` checks {checked}. \
         Add the missing decoder to `check_all`.\nFound:\n  {}",
        found.len(),
        found.join("\n  ")
    );
}

/// Production `impl <Trait><..> for ..` headers under `src/`, one bucket per
/// entry in `headers`.
///
/// Takes every header at once so the tree is read once rather than once per
/// trait. Matching on the header's own prefix excludes generic bounds, which
/// read `impl<T: TickDecoder<T>> ..` and are not impls of the trait.
fn collect_impls(headers: &[&str]) -> Vec<Vec<String>> {
    let mut found = vec![Vec::new(); headers.len()];

    crate::common::test_utils::source_scan::visit_production_sources(&mut |path, contents| {
        for line in contents.lines() {
            let line = line.trim_start();
            for (bucket, header) in found.iter_mut().zip(headers) {
                if line.starts_with(header) {
                    bucket.push(format!("{}: {line}", path.display()));
                }
            }
        }
    });

    for bucket in &mut found {
        bucket.sort();
    }
    found
}
