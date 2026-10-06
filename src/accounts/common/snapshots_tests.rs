use super::*;

fn row(account: &str, tag: &str, value: &str, currency: &str) -> AccountSummaryResult {
    AccountSummaryResult::Summary(AccountSummary {
        account: account.to_string(),
        tag: tag.to_string(),
        value: value.to_string(),
        currency: currency.to_string(),
    })
}

#[test]
fn test_later_row_replaces_earlier_value_for_same_key() {
    let mut builder = SnapshotBuilder::default();

    builder.apply(row("DU1", "NetLiquidation", "100.0", "USD"));
    builder.apply(row("DU1", "NetLiquidation", "101.5", "USD"));
    let snapshot = builder.take();

    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot.get("DU1", "NetLiquidation", "USD").unwrap().value, "101.5");
}

#[test]
fn test_snapshot_keeps_rows_not_resent_and_separates_keys() {
    let mut builder = SnapshotBuilder::default();

    builder.apply(row("DU1", "NetLiquidation", "100.0", "USD"));
    builder.apply(row("DU1", "NetLiquidation", "90.0", "EUR"));
    builder.apply(row("DU2", "NetLiquidation", "50.0", "USD"));
    builder.apply(row("DU1", "BuyingPower", "400.0", "USD"));
    builder.take();
    builder.apply(row("DU1", "NetLiquidation", "102.0", "USD"));
    let snapshot = builder.take();

    let observed: Vec<_> = snapshot
        .iter()
        .map(|row| (row.account.as_str(), row.tag.as_str(), row.currency.as_str(), row.value.as_str()))
        .collect();
    assert_eq!(
        observed,
        vec![
            ("DU1", "BuyingPower", "USD", "400.0"),
            ("DU1", "NetLiquidation", "EUR", "90.0"),
            ("DU1", "NetLiquidation", "USD", "102.0"),
            ("DU2", "NetLiquidation", "USD", "50.0"),
        ]
    );
}

#[test]
fn test_pending_follows_rows_and_take() {
    let mut builder = SnapshotBuilder::default();
    assert!(!builder.has_pending());

    assert!(!builder.apply(row("DU1", "NetLiquidation", "100.0", "USD")));
    let pending_after_row = builder.has_pending();
    assert!(builder.apply(AccountSummaryResult::End));
    let pending_after_end = builder.has_pending();
    builder.take();

    assert!(pending_after_row);
    assert!(pending_after_end);
    assert!(!builder.has_pending());
}

#[test]
fn test_row_with_unchanged_value_is_not_pending() {
    let mut builder = SnapshotBuilder::default();
    builder.apply(row("DU1", "NetLiquidation", "100.0", "USD"));
    builder.take();

    builder.apply(row("DU1", "NetLiquidation", "100.0", "USD"));
    let pending_after_repeat = builder.has_pending();
    builder.apply(row("DU1", "NetLiquidation", "100.5", "USD"));
    let pending_after_change = builder.has_pending();

    assert!(!pending_after_repeat);
    assert!(pending_after_change);
}

#[test]
fn test_end_completes_first_snapshot_even_when_empty_then_only_with_changes() {
    let mut builder = SnapshotBuilder::default();

    let first_end = builder.apply(AccountSummaryResult::End);
    builder.take();
    let end_without_change = builder.apply(AccountSummaryResult::End);
    builder.apply(row("DU1", "NetLiquidation", "100.0", "USD"));
    let end_with_change = builder.apply(AccountSummaryResult::End);

    assert!(first_end);
    assert!(!end_without_change);
    assert!(end_with_change);
}
