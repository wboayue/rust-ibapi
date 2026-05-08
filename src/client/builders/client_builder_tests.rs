//! Unit tests for `ClientBuilder` — exercise the validation paths without a
//! gateway. Live-handshake assertions live in `integration/{sync,async}/tests/connection.rs`.

use crate::errors::Error;

fn assert_invalid_argument(err: Option<Error>, expected_substr: &str) {
    let err = err.expect("expected failure");
    assert!(
        matches!(&err, Error::InvalidArgument(m) if m.contains(expected_substr)),
        "expected InvalidArgument containing {expected_substr:?}, got {err:?}"
    );
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
}
