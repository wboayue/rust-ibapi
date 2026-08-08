//! Builder-shape tests: the setters must reach the encoder, and the defaults
//! must reach it as absent rather than as zero.
//!
//! The request assertions are the point — before #752 these five values were
//! positional arguments, and every caller in the tree passed `None` for four of
//! them.

use crate::common::test_utils::helpers::{decode_request_proto, request_message_count};
use crate::server_versions;
use crate::wsh::AutoFill;
use time::macros::date;

#[cfg(feature = "sync")]
mod sync_tests {
    use super::*;
    use crate::common::test_utils::helpers::create_blocking_test_client_with_version;

    #[test]
    fn bare_contract_request_sends_no_filters() {
        let (client, bus) = create_blocking_test_client_with_version(server_versions::WSH_EVENT_DATA_FILTERS_DATE);

        // No response is configured, so the fetch ends in UnexpectedEndOfStream —
        // the request is what this asserts on.
        let _ = client.wsh_event_data_by_contract(76792991).fetch();

        assert_eq!(request_message_count(&bus), 1);
        let request: crate::proto::WshEventDataRequest = decode_request_proto(&bus, 0);
        assert_eq!(request.con_id, Some(76792991));
        assert_eq!(request.start_date, None, "an unset date must be absent, not empty");
        assert_eq!(request.end_date, None);
        assert_eq!(request.total_limit, None);
        assert_eq!(request.fill_watchlist, None);
    }

    #[test]
    fn setters_reach_the_request() {
        let (client, bus) = create_blocking_test_client_with_version(server_versions::WSH_EVENT_DATA_FILTERS_DATE);

        let _ = client
            .wsh_event_data_by_contract(76792991)
            .starting(date!(2024 - 01 - 01))
            .ending(date!(2024 - 03 - 31))
            .limit(50)
            .auto_fill(AutoFill {
                competitors: true,
                portfolio: false,
                watchlist: true,
            })
            .fetch();

        let request: crate::proto::WshEventDataRequest = decode_request_proto(&bus, 0);
        assert_eq!(request.start_date.as_deref(), Some("20240101"));
        assert_eq!(request.end_date.as_deref(), Some("20240331"));
        assert_eq!(request.total_limit, Some(50));
        assert_eq!(request.fill_competitors, Some(true));
        assert_eq!(request.fill_portfolio, Some(false));
        assert_eq!(request.fill_watchlist, Some(true));
    }

    #[test]
    fn a_date_filter_needs_the_server_version_for_it() {
        // The version gate belongs to the setters, not the entry point: the bare
        // request works at WSHE_CALENDAR, adding a date does not.
        let (client, _bus) = create_blocking_test_client_with_version(server_versions::WSHE_CALENDAR);

        let err = client
            .wsh_event_data_by_contract(76792991)
            .starting(date!(2024 - 01 - 01))
            .fetch()
            .expect_err("a date filter must be version-gated");
        assert!(matches!(err, crate::Error::ServerVersion(..)), "got {err:?}");
    }

    #[test]
    fn filter_request_carries_the_filter_and_limit() {
        let (client, bus) = create_blocking_test_client_with_version(server_versions::WSH_EVENT_DATA_FILTERS_DATE);

        let _ = client.wsh_event_data_by_filter(r#"{"country":"US"}"#).limit(10).subscribe();

        let request: crate::proto::WshEventDataRequest = decode_request_proto(&bus, 0);
        assert_eq!(request.filter.as_deref(), Some(r#"{"country":"US"}"#));
        assert_eq!(request.total_limit, Some(10));
        assert_eq!(request.con_id, None, "a filter request carries no contract id");
    }
}

#[cfg(feature = "async")]
mod async_tests {
    use super::*;
    use crate::common::test_utils::helpers::create_test_client_with_version;

    #[tokio::test]
    async fn bare_contract_request_sends_no_filters() {
        let (client, bus) = create_test_client_with_version(server_versions::WSH_EVENT_DATA_FILTERS_DATE);

        let _ = client.wsh_event_data_by_contract(76792991).fetch().await;

        assert_eq!(request_message_count(&bus), 1);
        let request: crate::proto::WshEventDataRequest = decode_request_proto(&bus, 0);
        assert_eq!(request.con_id, Some(76792991));
        assert_eq!(request.start_date, None, "an unset date must be absent, not empty");
        assert_eq!(request.total_limit, None);
    }

    #[tokio::test]
    async fn setters_reach_the_request() {
        let (client, bus) = create_test_client_with_version(server_versions::WSH_EVENT_DATA_FILTERS_DATE);

        let _ = client
            .wsh_event_data_by_contract(76792991)
            .starting(date!(2024 - 01 - 01))
            .ending(date!(2024 - 03 - 31))
            .limit(50)
            .auto_fill(AutoFill {
                competitors: true,
                portfolio: false,
                watchlist: true,
            })
            .fetch()
            .await;

        let request: crate::proto::WshEventDataRequest = decode_request_proto(&bus, 0);
        assert_eq!(request.start_date.as_deref(), Some("20240101"));
        assert_eq!(request.end_date.as_deref(), Some("20240331"));
        assert_eq!(request.total_limit, Some(50));
        assert_eq!(request.fill_competitors, Some(true));
        assert_eq!(request.fill_portfolio, Some(false));
        assert_eq!(request.fill_watchlist, Some(true));
    }

    #[tokio::test]
    async fn a_date_filter_needs_the_server_version_for_it() {
        let (client, _bus) = create_test_client_with_version(server_versions::WSHE_CALENDAR);

        let err = client
            .wsh_event_data_by_contract(76792991)
            .starting(date!(2024 - 01 - 01))
            .fetch()
            .await
            .expect_err("a date filter must be version-gated");
        assert!(matches!(err, crate::Error::ServerVersion(..)), "got {err:?}");
    }

    #[tokio::test]
    async fn filter_request_carries_the_filter_and_limit() {
        let (client, bus) = create_test_client_with_version(server_versions::WSH_EVENT_DATA_FILTERS_DATE);

        let _ = client.wsh_event_data_by_filter(r#"{"country":"US"}"#).limit(10).subscribe().await;

        let request: crate::proto::WshEventDataRequest = decode_request_proto(&bus, 0);
        assert_eq!(request.filter.as_deref(), Some(r#"{"country":"US"}"#));
        assert_eq!(request.total_limit, Some(10));
        assert_eq!(request.con_id, None, "a filter request carries no contract id");
    }
}
