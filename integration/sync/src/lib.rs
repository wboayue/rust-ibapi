//! Helpers shared by the sync integration tests.

use ibapi::subscriptions::SubscriptionItem;
use ibapi::Error;

/// Count data items until the first terminal item (end of stream or error),
/// logging notices; returns the count and that terminal item.
pub fn read_to_terminal<T>(
    mut next: impl FnMut() -> Option<Result<SubscriptionItem<T>, Error>>,
) -> (usize, Option<Result<SubscriptionItem<T>, Error>>) {
    let mut count = 0;
    loop {
        match next() {
            Some(Ok(SubscriptionItem::Data(_))) => count += 1,
            Some(Ok(SubscriptionItem::Notice(notice))) => eprintln!("notice: {notice}"),
            other => return (count, other),
        }
    }
}
