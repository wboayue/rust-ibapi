use super::*;
use crate::common::test_utils::helpers::*;
use crate::messages::OutgoingMessages;
use crate::testdata::builders::accounts::{
    account_download_end, account_summary, account_summary_end, account_update_multi, account_update_multi_end, account_update_time, account_value,
    portfolio_value,
};
use crate::testdata::builders::ResponseProtoEncoder;
// Test data
const TEST_REQUEST_ID: i32 = 123;
const TEST_SERVER_VERSION: i32 = 151;

fn test_context() -> DecoderContext {
    DecoderContext::new(TEST_SERVER_VERSION, None)
}

mod account_summary_tests {
    use super::*;

    #[test]
    fn test_decode_account_summary() {
        let mut message = proto_response(
            IncomingMessages::AccountSummary,
            account_summary().tag("NetLiquidation").value("123456.78").currency("USD").encode_proto(),
        );

        let result = AccountSummaryResult::decode(&test_context(), &mut message).unwrap();

        match result {
            AccountSummaryResult::Summary(summary) => {
                assert_eq!(summary.account, TEST_ACCOUNT);
                assert_eq!(summary.tag, "NetLiquidation");
                assert_eq!(summary.value, "123456.78");
                assert_eq!(summary.currency, "USD");
            }
            _ => panic!("Expected Summary variant"),
        }
    }

    #[test]
    fn test_decode_account_summary_end() {
        let mut message = proto_response(IncomingMessages::AccountSummaryEnd, account_summary_end().encode_proto());

        let result = AccountSummaryResult::decode(&test_context(), &mut message).unwrap();

        assert!(matches!(result, AccountSummaryResult::End));
    }

    #[test]
    fn test_decode_error_message() {
        // Error on the same request_id channel surfaces as Error::Notice, not a
        // parse failure or "unexpected message" error (#434).
        let mut message = proto_error_response(123, 10089, "Requested market data is not subscribed");
        let err = AccountSummaryResult::decode(&test_context(), &mut message).unwrap_err();
        assert_tws_error_message(err, 10089, "not subscribed");
    }

    #[test]
    fn test_cancel_message() {
        let bytes = AccountSummaryResult::cancel_message(TEST_SERVER_VERSION, Some(TEST_REQUEST_ID), None).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::CancelAccountSummary);
    }

    #[test]
    fn test_cancel_message_no_request_id() {
        let result = AccountSummaryResult::cancel_message(TEST_SERVER_VERSION, None, None);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidArgument(_)));
    }

    #[test]
    fn test_response_message_ids() {
        assert_eq!(
            AccountSummaryResult::RESPONSE_MESSAGE_IDS,
            &[
                IncomingMessages::AccountSummary,
                IncomingMessages::AccountSummaryEnd,
                IncomingMessages::Error
            ]
        );
    }
}

mod pnl_tests {
    use super::*;
    use crate::testdata::builders::accounts::pnl;

    #[test]
    fn test_decode_pnl() {
        let mut message = proto_response(
            IncomingMessages::PnL,
            pnl()
                .daily_pnl(1234.56)
                .unrealized_pnl(Some(2345.67))
                .realized_pnl(Some(3456.78))
                .encode_proto(),
        );

        let result = PnL::decode(&test_context(), &mut message).unwrap();

        assert_eq!(result.daily_pnl, 1234.56);
        assert_eq!(result.unrealized_pnl, Some(2345.67));
        assert_eq!(result.realized_pnl, Some(3456.78));
    }

    #[test]
    fn test_cancel_message() {
        let bytes = PnL::cancel_message(TEST_SERVER_VERSION, Some(TEST_REQUEST_ID), None).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::CancelPnL);
    }

    #[test]
    fn test_cancel_message_no_request_id() {
        let result = PnL::cancel_message(TEST_SERVER_VERSION, None, None);

        assert!(result.is_err());
    }

    #[test]
    fn test_response_message_ids() {
        assert_eq!(PnL::RESPONSE_MESSAGE_IDS, &[IncomingMessages::PnL, IncomingMessages::Error]);
    }

    #[test]
    fn test_decode_error_message() {
        let mut message = proto_error_response(123, 10089, "Requested market data is not subscribed");
        let err = PnL::decode(&test_context(), &mut message).unwrap_err();
        assert_tws_error_message(err, 10089, "not subscribed");
    }
}

mod pnl_single_tests {
    use super::*;
    use crate::testdata::builders::accounts::pnl_single;

    #[test]
    fn test_decode_pnl_single() {
        let mut message = proto_response(
            IncomingMessages::PnLSingle,
            pnl_single()
                .position(100.0)
                .daily_pnl(1234.56)
                .unrealized_pnl(2345.67)
                .realized_pnl(3456.78)
                .value(4567.89)
                .encode_proto(),
        );

        let result = PnLSingle::decode(&test_context(), &mut message).unwrap();

        assert_eq!(result.position, 100.0);
        assert_eq!(result.daily_pnl, 1234.56);
        assert_eq!(result.unrealized_pnl, 2345.67);
        assert_eq!(result.realized_pnl, 3456.78);
        assert_eq!(result.value, 4567.89);
    }

    #[test]
    fn test_cancel_message() {
        let bytes = PnLSingle::cancel_message(TEST_SERVER_VERSION, Some(TEST_REQUEST_ID), None).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::CancelPnLSingle);
    }

    #[test]
    fn test_response_message_ids() {
        assert_eq!(PnLSingle::RESPONSE_MESSAGE_IDS, &[IncomingMessages::PnLSingle, IncomingMessages::Error]);
    }

    #[test]
    fn test_decode_error_message() {
        let mut message = proto_error_response(123, 10089, "Requested market data is not subscribed");
        let err = PnLSingle::decode(&test_context(), &mut message).unwrap_err();
        assert_tws_error_message(err, 10089, "not subscribed");
    }
}

mod position_update_tests {
    use super::*;
    use crate::testdata::builders::positions::{position, position_end};
    use crate::testdata::builders::ResponseProtoEncoder;

    #[test]
    fn test_decode_position() {
        let mut message = proto_response(
            IncomingMessages::Position,
            position()
                .account(TEST_ACCOUNT)
                .contract_id(12345)
                .symbol("AAPL")
                .exchange("NASDAQ")
                .position(100.0)
                .average_cost(50.25)
                .encode_proto(),
        );

        let result = PositionUpdate::decode(&test_context(), &mut message).unwrap();

        match result {
            PositionUpdate::Position(pos) => {
                assert_eq!(pos.account, TEST_ACCOUNT);
                assert_eq!(pos.contract.contract_id, 12345);
                assert_eq!(pos.position as i32, 100);
                assert_eq!(pos.average_cost, 50.25);
            }
            _ => panic!("Expected Position variant"),
        }
    }

    #[test]
    fn test_decode_position_end() {
        let mut message = proto_response(IncomingMessages::PositionEnd, position_end().encode_proto());

        let result = PositionUpdate::decode(&test_context(), &mut message).unwrap();

        assert!(matches!(result, PositionUpdate::PositionEnd));
    }

    #[test]
    fn test_cancel_message() {
        let bytes = PositionUpdate::cancel_message(TEST_SERVER_VERSION, None, None).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::CancelPositions);
    }

    #[test]
    fn test_response_message_ids() {
        assert_eq!(
            PositionUpdate::RESPONSE_MESSAGE_IDS,
            &[IncomingMessages::Position, IncomingMessages::PositionEnd, IncomingMessages::Error]
        );
    }

    #[test]
    fn test_decode_error_message() {
        let mut message = proto_error_response(123, 10089, "Requested market data is not subscribed");
        let err = PositionUpdate::decode(&test_context(), &mut message).unwrap_err();
        assert_tws_error_message(err, 10089, "not subscribed");
    }
}

mod position_update_multi_tests {
    use super::*;
    use crate::testdata::builders::positions::{position_multi, position_multi_end};

    #[test]
    fn test_decode_position_multi() {
        let mut message = proto_response(
            IncomingMessages::PositionMulti,
            position_multi()
                .contract_id(12345)
                .symbol("AAPL")
                .exchange("NASDAQ")
                .position(100.0)
                .average_cost(50.25)
                .model_code(TEST_MODEL_CODE)
                .encode_proto(),
        );

        let result = PositionUpdateMulti::decode(&test_context(), &mut message).unwrap();

        match result {
            PositionUpdateMulti::Position(pos) => {
                assert_eq!(pos.account, TEST_ACCOUNT);
                assert_eq!(pos.contract.contract_id, 12345);
                assert_eq!(pos.position as i32, 100);
                assert_eq!(pos.average_cost, 50.25);
                assert_eq!(pos.model_code, TEST_MODEL_CODE);
            }
            _ => panic!("Expected Position variant"),
        }
    }

    #[test]
    fn test_decode_position_multi_end() {
        let mut message = proto_response(IncomingMessages::PositionMultiEnd, position_multi_end().encode_proto());

        let result = PositionUpdateMulti::decode(&test_context(), &mut message).unwrap();

        assert!(matches!(result, PositionUpdateMulti::PositionEnd));
    }

    #[test]
    fn test_cancel_message() {
        let bytes = PositionUpdateMulti::cancel_message(TEST_SERVER_VERSION, Some(TEST_REQUEST_ID), None).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::CancelPositionsMulti);
    }

    #[test]
    fn test_cancel_message_no_request_id() {
        let result = PositionUpdateMulti::cancel_message(TEST_SERVER_VERSION, None, None);

        assert!(result.is_err());
    }

    #[test]
    fn test_response_message_ids() {
        assert_eq!(
            PositionUpdateMulti::RESPONSE_MESSAGE_IDS,
            &[
                IncomingMessages::PositionMulti,
                IncomingMessages::PositionMultiEnd,
                IncomingMessages::Error
            ]
        );
    }

    #[test]
    fn test_decode_error_message() {
        let mut message = proto_error_response(123, 10089, "Requested market data is not subscribed");
        let err = PositionUpdateMulti::decode(&test_context(), &mut message).unwrap_err();
        assert_tws_error_message(err, 10089, "not subscribed");
    }
}

mod account_update_tests {
    use super::*;

    #[test]
    fn test_decode_account_value() {
        let mut message = proto_response(
            IncomingMessages::AccountValue,
            account_value()
                .key("NetLiquidation")
                .value("123456.78")
                .currency("USD")
                .account(TEST_ACCOUNT)
                .encode_proto(),
        );

        let result = AccountUpdate::decode(&test_context(), &mut message).unwrap();

        match result {
            AccountUpdate::AccountValue(val) => {
                assert_eq!(val.key, "NetLiquidation");
                assert_eq!(val.value, "123456.78");
                assert_eq!(val.currency, "USD");
                assert_eq!(val.account, Some(TEST_ACCOUNT.to_string()));
            }
            _ => panic!("Expected AccountValue variant"),
        }
    }

    #[test]
    fn test_decode_portfolio_value() {
        let mut message = proto_response(IncomingMessages::PortfolioValue, portfolio_value().contract_id(12345).encode_proto());

        let result = AccountUpdate::decode(&test_context(), &mut message).unwrap();

        match result {
            AccountUpdate::PortfolioValue(val) => {
                assert_eq!(val.contract.contract_id, 12345);
                assert_eq!(val.position as i32, 100);
                assert_eq!(val.market_price, 155.0);
                assert_eq!(val.market_value, 15500.0);
                assert_eq!(val.account, Some(TEST_ACCOUNT.to_string()));
            }
            _ => panic!("Expected PortfolioValue variant"),
        }
    }

    #[test]
    fn test_decode_update_time() {
        let mut message = proto_response(
            IncomingMessages::AccountUpdateTime,
            account_update_time().timestamp("14:30:00").encode_proto(),
        );

        let result = AccountUpdate::decode(&test_context(), &mut message).unwrap();

        match result {
            AccountUpdate::UpdateTime(time) => {
                assert_eq!(time.timestamp, "14:30:00");
            }
            _ => panic!("Expected UpdateTime variant"),
        }
    }

    #[test]
    fn test_decode_account_download_end() {
        let mut message = proto_response(IncomingMessages::AccountDownloadEnd, account_download_end().encode_proto());

        let result = AccountUpdate::decode(&test_context(), &mut message).unwrap();

        assert!(matches!(result, AccountUpdate::End));
    }

    #[test]
    fn test_cancel_message() {
        let bytes = AccountUpdate::cancel_message(TEST_SERVER_VERSION, None, None).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::RequestAccountData);
    }

    #[test]
    fn test_response_message_ids() {
        assert_eq!(
            AccountUpdate::RESPONSE_MESSAGE_IDS,
            &[
                IncomingMessages::AccountValue,
                IncomingMessages::PortfolioValue,
                IncomingMessages::AccountUpdateTime,
                IncomingMessages::AccountDownloadEnd,
                IncomingMessages::Error,
            ]
        );
    }

    #[test]
    fn test_decode_error_message() {
        let mut message = proto_error_response(123, 10089, "Requested market data is not subscribed");
        let err = AccountUpdate::decode(&test_context(), &mut message).unwrap_err();
        assert_tws_error_message(err, 10089, "not subscribed");
    }
}

mod account_update_multi_tests {
    use super::*;

    #[test]
    fn test_decode_account_multi_value() {
        let mut message = proto_response(
            IncomingMessages::AccountUpdateMulti,
            account_update_multi()
                .model_code(TEST_MODEL_CODE)
                .key("NetLiquidation")
                .value("123456.78")
                .currency("USD")
                .encode_proto(),
        );

        let result = AccountUpdateMulti::decode(&test_context(), &mut message).unwrap();

        match result {
            AccountUpdateMulti::AccountMultiValue(val) => {
                assert_eq!(val.account, TEST_ACCOUNT);
                assert_eq!(val.model_code, TEST_MODEL_CODE);
                assert_eq!(val.key, "NetLiquidation");
                assert_eq!(val.value, "123456.78");
                assert_eq!(val.currency, "USD");
            }
            _ => panic!("Expected AccountMultiValue variant"),
        }
    }

    #[test]
    fn test_decode_account_multi_end() {
        let mut message = proto_response(IncomingMessages::AccountUpdateMultiEnd, account_update_multi_end().encode_proto());

        let result = AccountUpdateMulti::decode(&test_context(), &mut message).unwrap();

        assert!(matches!(result, AccountUpdateMulti::End));
    }

    #[test]
    fn test_cancel_message() {
        let bytes = AccountUpdateMulti::cancel_message(TEST_SERVER_VERSION, Some(TEST_REQUEST_ID), None).unwrap();
        assert_proto_msg_id(&bytes, OutgoingMessages::CancelAccountUpdatesMulti);
    }

    #[test]
    fn test_cancel_message_no_request_id() {
        let result = AccountUpdateMulti::cancel_message(TEST_SERVER_VERSION, None, None);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidArgument(_)));
    }

    #[test]
    fn test_response_message_ids() {
        assert_eq!(
            AccountUpdateMulti::RESPONSE_MESSAGE_IDS,
            &[
                IncomingMessages::AccountUpdateMulti,
                IncomingMessages::AccountUpdateMultiEnd,
                IncomingMessages::Error
            ]
        );
    }

    #[test]
    fn test_decode_error_message() {
        let mut message = proto_error_response(123, 10089, "Requested market data is not subscribed");
        let err = AccountUpdateMulti::decode(&test_context(), &mut message).unwrap_err();
        assert_tws_error_message(err, 10089, "not subscribed");
    }
}

// Edge case tests
mod edge_cases {
    use super::*;

    #[test]
    fn test_empty_message_handling() {
        // Invalid message with missing fields
        let mut message = ResponseMessage::from("63\01\0123\0");

        let result = AccountSummaryResult::decode(&test_context(), &mut message);

        assert!(result.is_err());
    }

    #[test]
    fn test_malformed_message() {
        // Create a message with missing fields (only has account, missing tag, value, currency)
        let mut message = ResponseMessage::from("63\01\0123\0DU1234567\0");

        let result = AccountSummaryResult::decode(&test_context(), &mut message);

        assert!(result.is_err());
    }

    #[test]
    fn test_context_parameter_ignored() {
        // All cancel_message implementations should ignore the context parameter
        let context = DecoderContext::new(TEST_SERVER_VERSION, None).with_request_type(OutgoingMessages::RequestMarketData);

        // Test that context is ignored (should produce same result with or without)
        let result1 = AccountSummaryResult::cancel_message(TEST_SERVER_VERSION, Some(TEST_REQUEST_ID), None).unwrap();
        let result2 = AccountSummaryResult::cancel_message(TEST_SERVER_VERSION, Some(TEST_REQUEST_ID), Some(&context)).unwrap();

        // Both should produce identical messages
        assert_eq!(result1, result2);
    }
}

// Integration tests with actual decoder functions
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_account_summary_flow() {
        let messages = vec![
            proto_response(
                IncomingMessages::AccountSummary,
                account_summary().tag("NetLiquidation").value("100000.00").currency("USD").encode_proto(),
            ),
            proto_response(
                IncomingMessages::AccountSummary,
                account_summary().tag("TotalCashValue").value("50000.00").currency("USD").encode_proto(),
            ),
            proto_response(IncomingMessages::AccountSummaryEnd, account_summary_end().encode_proto()),
        ];

        let mut results = Vec::new();
        for mut message in messages {
            let result = AccountSummaryResult::decode(&test_context(), &mut message).unwrap();
            results.push(result);
        }

        assert_eq!(results.len(), 3);
        assert!(matches!(&results[0], AccountSummaryResult::Summary(_)));
        assert!(matches!(&results[1], AccountSummaryResult::Summary(_)));
        assert!(matches!(&results[2], AccountSummaryResult::End));
    }

    #[test]
    fn test_full_position_update_flow() {
        use crate::testdata::builders::positions::{position, position_end};
        use crate::testdata::builders::ResponseProtoEncoder;

        let messages = vec![
            proto_response(
                IncomingMessages::Position,
                position().account(TEST_ACCOUNT).symbol("AAPL").position(100.0).encode_proto(),
            ),
            proto_response(
                IncomingMessages::Position,
                position().account(TEST_ACCOUNT_2).symbol("GOOGL").position(200.0).encode_proto(),
            ),
            proto_response(IncomingMessages::PositionEnd, position_end().encode_proto()),
        ];

        let mut results = Vec::new();
        for mut message in messages {
            let result = PositionUpdate::decode(&test_context(), &mut message).unwrap();
            results.push(result);
        }

        assert_eq!(results.len(), 3);
        assert!(matches!(&results[0], PositionUpdate::Position(_)));
        assert!(matches!(&results[1], PositionUpdate::Position(_)));
        assert!(matches!(&results[2], PositionUpdate::PositionEnd));
    }
}
