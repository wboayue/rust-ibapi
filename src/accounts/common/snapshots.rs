//! Folds account summary rows into snapshots, shared by the sync and async clients.

use super::super::{AccountSummary, AccountSummaryResult, AccountSummarySnapshot};

/// Accumulates rows into a snapshot and tracks whether a row changed a value since the last
/// snapshot was taken.
#[derive(Debug, Default)]
pub(in crate::accounts) struct SnapshotBuilder {
    snapshot: AccountSummarySnapshot,
    pending: bool,
    emitted: bool,
}

impl SnapshotBuilder {
    /// Applies one subscription item. Returns `true` when it was an `End` marker that completes a
    /// snapshot: one with changed rows since the last snapshot, or the first one, which may be empty.
    pub(in crate::accounts) fn apply(&mut self, result: AccountSummaryResult) -> bool {
        match result {
            AccountSummaryResult::Summary(summary) => {
                self.insert(summary);
                false
            }
            AccountSummaryResult::End => self.pending || !self.emitted,
        }
    }

    /// Returns `true` when a row changed a value since the last snapshot was taken.
    pub(in crate::accounts) fn has_pending(&self) -> bool {
        self.pending
    }

    /// Returns a copy of the current snapshot and clears the pending flag.
    pub(in crate::accounts) fn take(&mut self) -> AccountSummarySnapshot {
        self.pending = false;
        self.emitted = true;
        self.snapshot.clone()
    }

    fn insert(&mut self, summary: AccountSummary) {
        let key = (summary.account.clone(), summary.tag.clone(), summary.currency.clone());
        let changed = self.snapshot.rows.get(&key) != Some(&summary);

        if changed {
            self.snapshot.rows.insert(key, summary);
            self.pending = true;
        }
    }
}

#[cfg(test)]
#[path = "snapshots_tests.rs"]
mod tests;
