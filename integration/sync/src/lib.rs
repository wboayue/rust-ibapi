//! Helpers shared by the sync integration tests.

use std::sync::LazyLock;

use ibapi::client::blocking::{Client, ClientBuilder};
use ibapi::subscriptions::SubscriptionItem;
use ibapi::{Error, RateLimiter};
use ibapi_test::{ClientId, GATEWAY};

/// One message budget for every client this test binary connects.
static LIMITER: LazyLock<RateLimiter> = LazyLock::new(RateLimiter::default);

/// A builder for the test gateway with the shared rate limiter installed.
pub fn client_builder(client_id: &ClientId) -> ClientBuilder {
    Client::builder().address(GATEWAY).client_id(client_id.id()).rate_limiter(LIMITER.clone())
}

/// Connect to the test gateway with the shared rate limiter installed.
pub fn connect(client_id: &ClientId) -> Client {
    client_builder(client_id).connect().expect("connection failed")
}

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
