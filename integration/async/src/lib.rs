//! Helpers shared by the async integration tests.

use futures::StreamExt;
use ibapi::subscriptions::SubscriptionItem;
use ibapi::Error;

/// Count data items until the first terminal item (end of stream or error),
/// logging notices; returns the count and that terminal item.
pub async fn read_to_terminal<T>(
    subscription: &mut (impl futures::Stream<Item = Result<SubscriptionItem<T>, Error>> + Unpin),
) -> (usize, Option<Result<SubscriptionItem<T>, Error>>) {
    let mut count = 0;
    loop {
        match subscription.next().await {
            Some(Ok(SubscriptionItem::Data(_))) => count += 1,
            Some(Ok(SubscriptionItem::Notice(notice))) => eprintln!("notice: {notice}"),
            other => return (count, other),
        }
    }
}
