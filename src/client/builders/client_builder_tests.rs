//! Unit tests for `ClientBuilder` — exercise the validation paths without a
//! gateway. Live-handshake assertions live in `integration/{sync,async}/tests/connection.rs`.

use crate::errors::Error;
use crate::transport::common::MAX_RECONNECT_ATTEMPTS;
use crate::transport::rate_limiter::RateLimiter;

use super::BuilderState;

fn assert_invalid_argument(err: Option<Error>, expected_substr: &str) {
    let err = err.expect("expected failure");
    assert!(
        matches!(&err, Error::InvalidArgument(m) if m.contains(expected_substr)),
        "expected InvalidArgument containing {expected_substr:?}, got {err:?}"
    );
}

#[test]
fn validate_passes_reconnect_limit_through() {
    let base = || BuilderState {
        address: Some("127.0.0.1:4002".into()),
        client_id: Some(100),
        ..Default::default()
    };

    let pieces = base().validate().expect("validate");
    assert_eq!(pieces.max_reconnect_attempts, Some(MAX_RECONNECT_ATTEMPTS));

    let mut limited = base();
    limited.max_reconnect_attempts = Some(50);
    assert_eq!(limited.validate().expect("validate").max_reconnect_attempts, Some(50));

    let mut unlimited = base();
    unlimited.max_reconnect_attempts = None;
    assert_eq!(unlimited.validate().expect("validate").max_reconnect_attempts, None);
}

fn connectable() -> BuilderState {
    BuilderState {
        address: Some("127.0.0.1:4002".into()),
        client_id: Some(100),
        ..Default::default()
    }
}

#[test]
fn validate_passes_rate_limiter_through() {
    assert!(
        connectable().validate().expect("validate").rate_limiter.is_none(),
        "limiter on by default"
    );

    // The pieces carry the caller's limiter, not a copy with its own budget.
    let limiter = RateLimiter::per_second(10);
    let mut state = connectable();
    state.rate_limiter = Some(limiter.clone());
    let passed = state.validate().expect("validate").rate_limiter.expect("limiter dropped");
    passed.reserve(std::time::Instant::now(), false);
    assert_eq!(limiter.reserved_until(), passed.reserved_until());
}

#[test]
fn validate_rejects_zero_rate_limit() {
    let mut state = connectable();
    state.rate_limiter = Some(RateLimiter::per_second(0));
    assert_invalid_argument(state.validate().err(), "rate_limiter");
}

#[cfg(feature = "sync")]
mod sync_tests {
    use super::super::sync_impl::ClientBuilder;
    use super::assert_invalid_argument;

    #[test]
    fn connect_without_address_returns_invalid_argument() {
        let result = ClientBuilder::default().client_id(100).connect();
        assert_invalid_argument(result.err(), "address");
    }

    #[test]
    fn connect_without_client_id_returns_invalid_argument() {
        let result = ClientBuilder::default().address("127.0.0.1:4002").connect();
        assert_invalid_argument(result.err(), "client_id");
    }

    #[test]
    fn reconnect_limit_configurators_set_state() {
        assert_eq!(
            ClientBuilder::default().state.max_reconnect_attempts,
            Some(crate::transport::common::MAX_RECONNECT_ATTEMPTS)
        );
        assert_eq!(ClientBuilder::default().max_reconnect_attempts(50).state.max_reconnect_attempts, Some(50));
        assert_eq!(ClientBuilder::default().reconnect_forever().state.max_reconnect_attempts, None);
    }

    #[test]
    fn tcp_no_delay_defaults_on() {
        assert!(ClientBuilder::default().state.tcp_no_delay);
        assert!(!ClientBuilder::default().tcp_no_delay(false).state.tcp_no_delay);
    }

    #[test]
    fn rate_limiter_configurator_sets_state() {
        assert!(ClientBuilder::default().state.rate_limiter.is_none());
        let builder = ClientBuilder::default().rate_limiter(crate::RateLimiter::per_second(7));
        assert_eq!(builder.state.rate_limiter.map(|l| l.messages_per_second()), Some(7));
    }
}

#[cfg(feature = "async")]
mod async_tests {
    use super::super::async_impl::ClientBuilder;
    use super::assert_invalid_argument;

    #[tokio::test]
    async fn connect_without_address_returns_invalid_argument() {
        let result = ClientBuilder::default().client_id(100).connect().await;
        assert_invalid_argument(result.err(), "address");
    }

    #[tokio::test]
    async fn connect_without_client_id_returns_invalid_argument() {
        let result = ClientBuilder::default().address("127.0.0.1:4002").connect().await;
        assert_invalid_argument(result.err(), "client_id");
    }

    #[tokio::test]
    async fn connect_with_zero_channel_capacity_returns_invalid_argument() {
        // tokio's broadcast::channel(0) panics; the builder must reject it
        // at the validation seam instead.
        let result = ClientBuilder::default()
            .address("127.0.0.1:4002")
            .client_id(100)
            .channel_capacity(0)
            .connect()
            .await;
        assert_invalid_argument(result.err(), "channel_capacity");
    }

    #[test]
    fn reconnect_limit_configurators_set_state() {
        assert_eq!(
            ClientBuilder::default().state.max_reconnect_attempts,
            Some(crate::transport::common::MAX_RECONNECT_ATTEMPTS)
        );
        assert_eq!(ClientBuilder::default().max_reconnect_attempts(50).state.max_reconnect_attempts, Some(50));
        assert_eq!(ClientBuilder::default().reconnect_forever().state.max_reconnect_attempts, None);
    }

    #[test]
    fn tcp_no_delay_defaults_on() {
        assert!(ClientBuilder::default().state.tcp_no_delay);
        assert!(!ClientBuilder::default().tcp_no_delay(false).state.tcp_no_delay);
    }

    #[test]
    fn rate_limiter_configurator_sets_state() {
        assert!(ClientBuilder::default().state.rate_limiter.is_none());
        let builder = ClientBuilder::default().rate_limiter(crate::RateLimiter::per_second(7));
        assert_eq!(builder.state.rate_limiter.map(|l| l.messages_per_second()), Some(7));
    }
}
