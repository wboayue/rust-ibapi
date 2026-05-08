//! Unit tests for `ClientBuilder` — exercise the validation paths without a
//! gateway. Live-handshake assertions live in `integration/{sync,async}/tests/connection.rs`.

#[cfg(feature = "sync")]
mod sync_tests {
    use super::super::sync_impl::ClientBuilder;
    use crate::errors::Error;

    fn assert_invalid_argument(err: Option<Error>, expected_substr: &str) {
        let err = err.expect("expected failure");
        assert!(
            matches!(&err, Error::InvalidArgument(m) if m.contains(expected_substr)),
            "expected InvalidArgument containing {expected_substr:?}, got {err:?}"
        );
    }

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
    fn connect_with_notice_stream_without_address_returns_invalid_argument() {
        // Result has a non-Debug T `(Client, NoticeStream)`, so we extract the
        // Err branch via match instead of expect_err.
        let result = ClientBuilder::default().client_id(100).connect_with_notice_stream();
        let err = match result {
            Ok(_) => panic!("expected failure"),
            Err(e) => e,
        };
        assert!(matches!(&err, Error::InvalidArgument(m) if m.contains("address")), "got: {err:?}");
    }
}

#[cfg(feature = "async")]
mod async_tests {
    use super::super::async_impl::ClientBuilder;
    use crate::errors::Error;

    fn assert_invalid_argument(err: Option<Error>, expected_substr: &str) {
        let err = err.expect("expected failure");
        assert!(
            matches!(&err, Error::InvalidArgument(m) if m.contains(expected_substr)),
            "expected InvalidArgument containing {expected_substr:?}, got {err:?}"
        );
    }

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
    async fn connect_with_notice_stream_without_address_returns_invalid_argument() {
        // Result has a non-Debug T `(Client, NoticeStream)`, so we extract the
        // Err branch via match instead of expect_err.
        let result = ClientBuilder::default().client_id(100).connect_with_notice_stream().await;
        let err = match result {
            Ok(_) => panic!("expected failure"),
            Err(e) => e,
        };
        assert!(matches!(&err, Error::InvalidArgument(m) if m.contains("address")), "got: {err:?}");
    }
}
