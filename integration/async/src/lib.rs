//! Helpers shared by the async integration tests.

use std::sync::LazyLock;

use futures::StreamExt;
use ibapi::subscriptions::SubscriptionItem;
use ibapi::{Client, ClientBuilder, Error, RateLimiter};
use ibapi_test::{ClientId, GATEWAY};

/// One message budget for every client this test binary connects.
static LIMITER: LazyLock<RateLimiter> = LazyLock::new(RateLimiter::default);

/// A builder for the test gateway with the shared rate limiter installed.
pub fn client_builder(client_id: &ClientId) -> ClientBuilder {
    Client::builder().address(GATEWAY).client_id(client_id.id()).rate_limiter(LIMITER.clone())
}

/// Connect to the test gateway with the shared rate limiter installed.
pub async fn connect(client_id: &ClientId) -> Client {
    client_builder(client_id).connect().await.expect("connection failed")
}

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
