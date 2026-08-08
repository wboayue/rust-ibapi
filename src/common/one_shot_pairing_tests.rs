//! Cross-check of every [`expect_proto`](super::expect_proto) call site's
//! `(IncomingMessages, decode_*_proto)` pair against a declared roster.
//!
//! `expect_proto`'s two arguments are unrelated to the compiler: `R` is inferred
//! from the decoder and `expected` is a free runtime value, so
//! `expect_proto(IncomingMessages::UserInfo, decode_family_codes_proto)` builds
//! and runs. It would feed a `UserInfo` frame's bytes to `prost::FamilyCodes`,
//! and the only signal is a decode error at runtime against a live gateway.
//!
//! This is the one-shot analogue of `test_decoder_roster_is_complete`
//! (`src/subscriptions/response_message_ids_tests.rs`): the pair is a property
//! of the proto payload, not of the caller, but it is currently spelled at the
//! call site — twice, since sync and async each carry one. So the roster below
//! is the single source, and both directions are checked:
//!
//! - a site whose pair is absent from the roster fails, which is what a
//!   mispairing looks like;
//! - a roster entry no site uses fails, so the roster cannot rot;
//! - each pair must appear exactly [`SITES_PER_PAIR`] times, which catches a
//!   sync/async pair drifting apart — the direction no per-API test sees,
//!   because sync and async tests each exercise only their own side.
//!
//! Adding a one-shot API means adding a line here. That is the cost of keeping
//! the pairing outside the type system; see the follow-up in
//! `plans/claude-md-knowledge-graph.md` for the `ProtoPayload` trait that would
//! retire this file.

use std::collections::BTreeMap;
use std::path::Path;

/// Sync and async each spell the pair once, and neither is allowed to drift.
const SITES_PER_PAIR: usize = 2;

/// Every legal `(IncomingMessages variant, proto decoder)` pairing.
///
/// Most read as the same name in two cases. The ones that do not are real:
/// TWS's message name and our decoder's name diverge for `CurrentTime` /
/// `server_time`, `CurrentTimeInMillis` / `server_time_millis`,
/// `MktDepthExchanges` / `market_depth_exchanges`, `ConfigResponse` / `config`,
/// and `UpdateConfigResponse` / `update_config`. That is why this roster is a
/// table and not a name-derivation rule.
const PAIRS: &[(&str, &str)] = &[
    ("ConfigResponse", "decode_config_proto"),
    ("CurrentTime", "decode_server_time_proto"),
    ("CurrentTimeInMillis", "decode_server_time_millis_proto"),
    ("FamilyCodes", "decode_family_codes_proto"),
    ("HeadTimestamp", "decode_head_timestamp_proto"),
    ("HistogramData", "decode_histogram_data_proto"),
    ("HistoricalSchedule", "decode_historical_schedule_proto"),
    ("ManagedAccounts", "decode_managed_accounts_proto"),
    ("MarketRule", "decode_market_rule_proto"),
    ("MktDepthExchanges", "decode_market_depth_exchanges_proto"),
    ("NewsArticle", "decode_news_article_proto"),
    ("NewsProviders", "decode_news_providers_proto"),
    ("NextValidId", "decode_next_valid_id_proto"),
    ("ReceiveFA", "decode_receive_fa_proto"),
    ("ReplaceFAEnd", "decode_replace_fa_end_proto"),
    ("ScannerParameters", "decode_scanner_parameters_proto"),
    ("SmartComponents", "decode_smart_components_proto"),
    ("SoftDollarTier", "decode_soft_dollar_tiers_proto"),
    ("SymbolSamples", "decode_symbol_samples_proto"),
    ("UpdateConfigResponse", "decode_update_config_proto"),
    ("UserInfo", "decode_user_info_proto"),
    ("VerifyCompleted", "decode_verify_completed_proto"),
    ("VerifyMessageApi", "decode_verify_message_api_proto"),
    ("WshEventData", "decode_wsh_event_data_proto"),
    ("WshMetaData", "decode_wsh_metadata_proto"),
];

/// Scrape `(variant, decoder)` out of every `expect_proto(..)` in production
/// source under `src/`, skipping test files.
fn collect_sites(dir: &Path, found: &mut Vec<(String, String, String)>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_sites(&path, found);
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if !name.ends_with(".rs") || name == "tests.rs" || name.ends_with("_tests.rs") {
            continue;
        }
        let contents = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        for (variant, decoder) in parse_sites(&contents) {
            found.push((path.display().to_string(), variant, decoder));
        }
    }
}

/// `expect_proto(IncomingMessages::X, [path::]decode_y_proto)` → `("X", "decode_y_proto")`.
///
/// Whitespace is collapsed first: rustfmt wraps the longer call sites across
/// lines, and a line-oriented scraper would skip exactly those — silently, which
/// is the failure mode a gate must not have. (`SITES_PER_PAIR` would still catch
/// it as a count of 1, but a gate should not depend on its own backstop.)
fn parse_sites(contents: &str) -> Vec<(String, String)> {
    const OPEN: &str = "expect_proto(IncomingMessages::";
    let flat = contents.split_whitespace().collect::<Vec<_>>().join(" ").replace("( ", "(");
    let mut out = Vec::new();
    for (start, _) in flat.match_indices(OPEN) {
        let rest = &flat[start + OPEN.len()..];
        let Some((variant, rest)) = rest.split_once(',') else { continue };
        let Some((decoder, _)) = rest.split_once([')', ',']) else { continue };
        let decoder = decoder.trim();
        // Skip the closure-argument form used by request_helpers' own tests.
        if !decoder.ends_with("_proto") {
            continue;
        }
        let decoder = decoder.rsplit("::").next().unwrap_or(decoder);
        out.push((variant.to_string(), decoder.to_string()));
    }
    out
}

#[test]
fn test_expect_proto_sites_match_the_roster() {
    let mut sites = Vec::new();
    collect_sites(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut sites);
    assert!(
        !sites.is_empty(),
        "scraper found no expect_proto sites — the parser is broken, not the code"
    );

    let mut counts: BTreeMap<(String, String), usize> = BTreeMap::new();
    for (file, variant, decoder) in &sites {
        assert!(
            PAIRS.contains(&(variant.as_str(), decoder.as_str())),
            "{file}: expect_proto(IncomingMessages::{variant}, {decoder}) is not a declared pairing. \
             Either the variant and the decoder disagree — which would feed one message's bytes to \
             another's prost type — or this is a new one-shot API and PAIRS needs a line."
        );
        *counts.entry((variant.clone(), decoder.clone())).or_default() += 1;
    }

    for (variant, decoder) in PAIRS {
        let n = counts.get(&(variant.to_string(), decoder.to_string())).copied().unwrap_or(0);
        assert_eq!(
            n, SITES_PER_PAIR,
            "IncomingMessages::{variant} / {decoder} is used at {n} site(s), expected {SITES_PER_PAIR} \
             (one sync, one async). Zero means the roster entry is stale; one means sync and async \
             have drifted apart."
        );
    }
}

#[test]
fn test_parse_sites_reads_both_call_shapes() {
    // Qualified and bare decoder paths, and a multi-line call as rustfmt writes it.
    let src = r#"
        expect_proto(IncomingMessages::UserInfo, decoders::decode_user_info_proto),
        expect_proto(
            IncomingMessages::HeadTimestamp,
            decode_head_timestamp_proto,
        )
        expect_proto(IncomingMessages::CurrentTime, |bytes: &[u8]| Ok(bytes.len()));
    "#;
    let found = parse_sites(src);
    assert_eq!(
        found,
        vec![
            ("UserInfo".to_string(), "decode_user_info_proto".to_string()),
            ("HeadTimestamp".to_string(), "decode_head_timestamp_proto".to_string()),
        ],
        "both the single-line and rustfmt-wrapped forms must be read; the closure form is skipped"
    );
}
