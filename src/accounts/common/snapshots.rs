//! Folds account summary rows into snapshots, shared by the sync and async clients.

use super::super::{AccountSummary, AccountSummaryResult, AccountSummarySnapshot};

/// Accumulates rows into a snapshot and tracks whether rows arrived since the last one was taken.
#[derive(Debug, Default)]
pub(in crate::accounts) struct SnapshotBuilder {
    snapshot: AccountSummarySnapshot,
    pending: bool,
}

impl SnapshotBuilder {
    /// Applies one subscription item. Returns `true` when it was an `End` marker, which completes
    /// a snapshot.
    pub(in crate::accounts) fn apply(&mut self, result: AccountSummaryResult) -> bool {
        match result {
            AccountSummaryResult::Summary(summary) => {
                self.insert(summary);
                self.pending = true;
                false
            }
            AccountSummaryResult::End => true,
        }
    }

    /// Returns `true` when rows arrived since the last snapshot was taken.
    pub(in crate::accounts) fn has_pending(&self) -> bool {
        self.pending
    }

    /// Returns a copy of the current snapshot and clears the pending flag.
    pub(in crate::accounts) fn take(&mut self) -> AccountSummarySnapshot {
        self.pending = false;
        self.snapshot.clone()
    }

    fn insert(&mut self, summary: AccountSummary) {
        let key = (summary.account.clone(), summary.tag.clone(), summary.currency.clone());
        self.snapshot.rows.insert(key, summary);
    }
}

#[cfg(test)]
mod tests {
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
}
