use super::*;
use crate::accounts::AccountSummary;

fn row(account: &str, tag: &str, value: &str, currency: &str) -> AccountSummaryResult {
    AccountSummaryResult::Summary(AccountSummary {
        account: account.to_string(),
        tag: tag.to_string(),
        value: value.to_string(),
        currency: currency.to_string(),
    })
}

type Batch = Vec<AccountSummaryResult>;

fn net_liq(value: &str) -> AccountSummaryResult {
    row("DU1", "NetLiquidation", value, "USD")
}

fn values(snapshot: &AccountSummarySnapshot) -> Vec<(&str, &str, &str, &str)> {
    snapshot
        .iter()
        .map(|row| (row.account.as_str(), row.tag.as_str(), row.currency.as_str(), row.value.as_str()))
        .collect()
}

#[test]
fn test_fold_keeps_rows_not_resent_and_separates_keys() {
    let mut builder = SnapshotBuilder::default();

    builder.fold(vec![
        row("DU1", "NetLiquidation", "100.0", "USD"),
        row("DU1", "NetLiquidation", "90.0", "EUR"),
        row("DU2", "NetLiquidation", "50.0", "USD"),
        row("DU1", "BuyingPower", "400.0", "USD"),
        AccountSummaryResult::End,
    ]);
    let snapshot = builder.fold(vec![net_liq("101.0"), net_liq("102.0")]).unwrap();

    assert_eq!(
        values(&snapshot),
        vec![
            ("DU1", "BuyingPower", "USD", "400.0"),
            ("DU1", "NetLiquidation", "EUR", "90.0"),
            ("DU1", "NetLiquidation", "USD", "102.0"),
            ("DU2", "NetLiquidation", "USD", "50.0"),
        ]
    );
}

#[test]
fn test_fold_emits_when_snapshot_completes() {
    // (name, earlier batches, batch under test, emits)
    let cases: Vec<(&str, Vec<Batch>, Batch, bool)> = vec![
        (
            "first batch ending in End",
            vec![],
            vec![net_liq("100.0"), AccountSummaryResult::End],
            true,
        ),
        ("first batch only End, empty", vec![], vec![AccountSummaryResult::End], true),
        ("first batch without End held", vec![], vec![net_liq("100.0")], false),
        (
            "rest of slow initial dump",
            vec![vec![net_liq("100.0")]],
            vec![AccountSummaryResult::End],
            true,
        ),
        (
            "push with a change",
            vec![vec![net_liq("100.0"), AccountSummaryResult::End]],
            vec![net_liq("100.5")],
            true,
        ),
        (
            "push without change",
            vec![vec![net_liq("100.0"), AccountSummaryResult::End]],
            vec![net_liq("100.0")],
            false,
        ),
        (
            "repeated End without change",
            vec![vec![net_liq("100.0"), AccountSummaryResult::End]],
            vec![net_liq("100.0"), AccountSummaryResult::End],
            false,
        ),
    ];

    for (name, earlier, batch, emits) in cases {
        let mut builder = SnapshotBuilder::default();
        for earlier in earlier {
            builder.fold(earlier);
        }

        assert_eq!(builder.fold(batch).is_some(), emits, "{name}");
    }
}

#[test]
fn test_flush_returns_rows_not_yet_emitted() {
    let mut builder = SnapshotBuilder::default();
    assert!(builder.flush().is_none());

    builder.fold(vec![net_liq("100.0")]);
    let flushed = builder.flush().unwrap();

    assert_eq!(values(&flushed), vec![("DU1", "NetLiquidation", "USD", "100.0")]);
    assert!(builder.flush().is_none());
}
