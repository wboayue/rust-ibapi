//! Builder-shape tests: the one setter must reach the encoder, and the default
//! must reach it as absent rather than as an empty string.
//!
//! The request assertions are the point — before this builder, `exchange` was a
//! positional `&str` that two examples filled with `"SMART"`, which TWS answers
//! with an empty chain for a stock underlying.

use crate::common::test_utils::helpers::{decode_request_proto, request_message_count};
use crate::contracts::SecurityType;
use crate::server_versions;

#[cfg(feature = "sync")]
mod sync_tests {
    use super::*;
    use crate::common::test_utils::helpers::create_blocking_test_client_with_version;

    #[test]
    fn bare_request_sends_no_exchange() {
        let (client, bus) = create_blocking_test_client_with_version(server_versions::SEC_DEF_OPT_PARAMS_REQ);

        let _ = client.option_chain("AAPL", SecurityType::Stock, 265598).subscribe();

        assert_eq!(request_message_count(&bus), 1);
        let request: crate::proto::SecDefOptParamsRequest = decode_request_proto(&bus, 0);
        assert_eq!(request.underlying_symbol.as_deref(), Some("AAPL"));
        assert_eq!(request.underlying_sec_type.as_deref(), Some("STK"));
        assert_eq!(request.underlying_con_id, Some(265598));
        assert_eq!(request.fut_fop_exchange, None, "an unset exchange must be absent, not empty");
    }

    #[test]
    fn exchange_setter_reaches_the_request() {
        let (client, bus) = create_blocking_test_client_with_version(server_versions::SEC_DEF_OPT_PARAMS_REQ);

        let _ = client.option_chain("ES", SecurityType::Future, 495512563).exchange("CME").subscribe();

        let request: crate::proto::SecDefOptParamsRequest = decode_request_proto(&bus, 0);
        assert_eq!(request.underlying_sec_type.as_deref(), Some("FUT"));
        assert_eq!(request.fut_fop_exchange.as_deref(), Some("CME"));
    }
}

#[cfg(feature = "async")]
mod async_tests {
    use super::*;
    use crate::common::test_utils::helpers::create_test_client_with_version;

    #[tokio::test]
    async fn bare_request_sends_no_exchange() {
        let (client, bus) = create_test_client_with_version(server_versions::SEC_DEF_OPT_PARAMS_REQ);

        let _ = client.option_chain("AAPL", SecurityType::Stock, 265598).subscribe().await;

        assert_eq!(request_message_count(&bus), 1);
        let request: crate::proto::SecDefOptParamsRequest = decode_request_proto(&bus, 0);
        assert_eq!(request.underlying_symbol.as_deref(), Some("AAPL"));
        assert_eq!(request.underlying_con_id, Some(265598));
        assert_eq!(request.fut_fop_exchange, None, "an unset exchange must be absent, not empty");
    }

    #[tokio::test]
    async fn exchange_setter_reaches_the_request() {
        let (client, bus) = create_test_client_with_version(server_versions::SEC_DEF_OPT_PARAMS_REQ);

        let _ = client
            .option_chain("ES", SecurityType::Future, 495512563)
            .exchange("CME")
            .subscribe()
            .await;

        let request: crate::proto::SecDefOptParamsRequest = decode_request_proto(&bus, 0);
        assert_eq!(request.fut_fop_exchange.as_deref(), Some("CME"));
    }
}
