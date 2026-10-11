//! Folds account summary batches into snapshots, shared by the sync and async clients.

use super::super::{AccountSummaryResult, AccountSummarySnapshot};
use crate::subscriptions::StreamDecoder;

/// Accumulates batches into a snapshot and decides when one is complete.
#[derive(Debug, Default)]
pub(in crate::accounts) struct SnapshotBuilder {
    snapshot: AccountSummarySnapshot,
    pending: bool,
    emitted: bool,
}

impl SnapshotBuilder {
    /// Applies one batch from [`next_batch`](crate::subscriptions::Subscription::next_batch).
    /// Returns a snapshot when it completes one: the first at the batch that ends in `End`, even
    /// when empty, so a slow initial dump split by the quiet period is held until its `End`; each
    /// later one at any batch that changed a value.
    pub(in crate::accounts) fn fold(&mut self, batch: Vec<AccountSummaryResult>) -> Option<AccountSummarySnapshot> {
        let ended = batch.last().is_some_and(StreamDecoder::is_batch_end);
        for result in batch {
            if let AccountSummaryResult::Summary(summary) = result {
                self.pending |= self.snapshot.apply(summary);
            }
        }

        let complete = if self.emitted { self.pending } else { ended };
        complete.then(|| self.take())
    }

    /// Returns the rows not yet emitted, for when the subscription ends.
    pub(in crate::accounts) fn flush(&mut self) -> Option<AccountSummarySnapshot> {
        self.pending.then(|| self.take())
    }

    fn take(&mut self) -> AccountSummarySnapshot {
        self.pending = false;
        self.emitted = true;
        self.snapshot.clone()
    }
}

#[cfg(test)]
#[path = "snapshots_tests.rs"]
mod tests;
