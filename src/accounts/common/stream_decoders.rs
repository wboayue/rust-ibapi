//! Common DataStream implementations for accounts module
//!
//! This module contains the DataStream trait implementations that are shared
//! between sync and async versions, avoiding code duplication.

use crate::accounts::*;
use crate::messages::{IncomingMessages, RequestMessage, ResponseMessage};
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::Error;

use super::{decoders, encoders};
use crate::common::error_helpers;

impl StreamDecoder<AccountSummaryResult> for AccountSummaryResult {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::AccountSummary, IncomingMessages::AccountSummaryEnd];

    fn decode(context: &DecoderContext, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::AccountSummary => Ok(AccountSummaryResult::Summary(decoders::decode_account_summary(
                context.server_version,
                message,
            )?)),
            IncomingMessages::AccountSummaryEnd => Ok(AccountSummaryResult::End),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<RequestMessage, Error> {
        let request_id = error_helpers::require_request_id(request_id)?;
        encoders::encode_cancel_account_summary(request_id)
    }
}

impl StreamDecoder<PnL> for PnL {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::PnL];

    fn decode(context: &DecoderContext, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_pnl(context.server_version, message)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<RequestMessage, Error> {
        let request_id = error_helpers::require_request_id(request_id)?;
        encoders::encode_cancel_pnl(request_id)
    }
}

impl StreamDecoder<PnLSingle> for PnLSingle {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::PnLSingle];

    fn decode(context: &DecoderContext, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_pnl_single(context.server_version, message)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<RequestMessage, Error> {
        let request_id = error_helpers::require_request_id(request_id)?;
        encoders::encode_cancel_pnl_single(request_id)
    }
}

impl StreamDecoder<PositionUpdate> for PositionUpdate {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::Position, IncomingMessages::PositionEnd];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::Position => Ok(PositionUpdate::Position(decoders::decode_position(message)?)),
            IncomingMessages::PositionEnd => Ok(PositionUpdate::PositionEnd),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(_server_version: i32, _request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<RequestMessage, Error> {
        encoders::encode_cancel_positions()
    }
}

impl StreamDecoder<PositionUpdateMulti> for PositionUpdateMulti {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::PositionMulti, IncomingMessages::PositionMultiEnd];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::PositionMulti => Ok(PositionUpdateMulti::Position(decoders::decode_position_multi(message)?)),
            IncomingMessages::PositionMultiEnd => Ok(PositionUpdateMulti::PositionEnd),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<RequestMessage, Error> {
        let request_id = error_helpers::require_request_id(request_id)?;
        encoders::encode_cancel_positions_multi(request_id)
    }
}

impl StreamDecoder<AccountUpdate> for AccountUpdate {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[
        IncomingMessages::AccountValue,
        IncomingMessages::PortfolioValue,
        IncomingMessages::AccountUpdateTime,
        IncomingMessages::AccountDownloadEnd,
    ];

    fn decode(context: &DecoderContext, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::AccountValue => Ok(AccountUpdate::AccountValue(decoders::decode_account_value(message)?)),
            IncomingMessages::PortfolioValue => Ok(AccountUpdate::PortfolioValue(decoders::decode_account_portfolio_value(
                context.server_version,
                message,
            )?)),
            IncomingMessages::AccountUpdateTime => Ok(AccountUpdate::UpdateTime(decoders::decode_account_update_time(message)?)),
            IncomingMessages::AccountDownloadEnd => Ok(AccountUpdate::End),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(server_version: i32, _request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<RequestMessage, Error> {
        encoders::encode_cancel_account_updates(server_version)
    }
}

impl StreamDecoder<AccountUpdateMulti> for AccountUpdateMulti {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::AccountUpdateMulti, IncomingMessages::AccountUpdateMultiEnd];

    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::AccountUpdateMulti => Ok(AccountUpdateMulti::AccountMultiValue(decoders::decode_account_multi_value(message)?)),
            IncomingMessages::AccountUpdateMultiEnd => Ok(AccountUpdateMulti::End),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(server_version: i32, request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<RequestMessage, Error> {
        let request_id = error_helpers::require_request_id_for(request_id, "encode cancel account updates multi")?;
        encoders::encode_cancel_account_updates_multi(server_version, request_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::test_utils::helpers::*;
    use crate::messages::OutgoingMessages;
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
            // Format: message_type\0version\0request_id\0account\0tag\0value\0currency\0
            let mut message = ResponseMessage::from("63\01\0123\0DU1234567\0NetLiquidation\0123456.78\0USD\0");

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
            // Format: message_type\0version\0request_id\0
            let mut message = ResponseMessage::from("64\01\0123\0");

            let result = AccountSummaryResult::decode(&test_context(), &mut message).unwrap();

            assert!(matches!(result, AccountSummaryResult::End));
        }

        #[test]
        fn test_decode_unexpected_message() {
            // Using Error message type which is not expected for AccountSummaryResult
            let mut message = ResponseMessage::from("4\02\0123\0Some error\0");

            let result = AccountSummaryResult::decode(&test_context(), &mut message);

            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("unexpected message"));
        }

        #[test]
        fn test_cancel_message() {
            let request = AccountSummaryResult::cancel_message(TEST_SERVER_VERSION, Some(TEST_REQUEST_ID), None).unwrap();

            assert_eq!(request[0], OutgoingMessages::CancelAccountSummary.to_string());
            assert_eq!(request[1], "1"); // version
            assert_eq!(request[2], TEST_REQUEST_ID.to_string());
        }

        #[test]
        fn test_cancel_message_no_request_id() {
            let result = AccountSummaryResult::cancel_message(TEST_SERVER_VERSION, None, None);

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), Error::Simple(_)));
        }

        #[test]
        fn test_response_message_ids() {
            assert_eq!(
                AccountSummaryResult::RESPONSE_MESSAGE_IDS,
                &[IncomingMessages::AccountSummary, IncomingMessages::AccountSummaryEnd]
            );
        }
    }

    mod pnl_tests {
        use super::*;

        #[test]
        fn test_decode_pnl() {
            // Format: message_type\0request_id\0daily_pnl\0unrealized_pnl\0realized_pnl\0
            let mut message = ResponseMessage::from("94\0123\01234.56\02345.67\03456.78\0");

            let result = PnL::decode(&test_context(), &mut message).unwrap();

            assert_eq!(result.daily_pnl, 1234.56);
            assert_eq!(result.unrealized_pnl, Some(2345.67));
            assert_eq!(result.realized_pnl, Some(3456.78));
        }

        #[test]
        fn test_cancel_message() {
            let request = PnL::cancel_message(TEST_SERVER_VERSION, Some(TEST_REQUEST_ID), None).unwrap();

            assert_eq!(request[0], OutgoingMessages::CancelPnL.to_string());
            assert_eq!(request[1], TEST_REQUEST_ID.to_string());
        }

        #[test]
        fn test_cancel_message_no_request_id() {
            let result = PnL::cancel_message(TEST_SERVER_VERSION, None, None);

            assert!(result.is_err());
        }

        #[test]
        fn test_response_message_ids() {
            assert_eq!(PnL::RESPONSE_MESSAGE_IDS, &[IncomingMessages::PnL]);
        }
    }

    mod pnl_single_tests {
        use super::*;

        #[test]
        fn test_decode_pnl_single() {
            // Format: message_type\0request_id\0position\0daily_pnl\0unrealized_pnl\0realized_pnl\0value\0
            let mut message = ResponseMessage::from("95\0123\0100\01234.56\02345.67\03456.78\04567.89\0");

            let result = PnLSingle::decode(&test_context(), &mut message).unwrap();

            assert_eq!(result.position, 100.0);
            assert_eq!(result.daily_pnl, 1234.56);
            assert_eq!(result.unrealized_pnl, 2345.67);
            assert_eq!(result.realized_pnl, 3456.78);
            assert_eq!(result.value, 4567.89);
        }

        #[test]
        fn test_cancel_message() {
            let request = PnLSingle::cancel_message(TEST_SERVER_VERSION, Some(TEST_REQUEST_ID), None).unwrap();

            assert_eq!(request[0], OutgoingMessages::CancelPnLSingle.to_string());
            assert_eq!(request[1], TEST_REQUEST_ID.to_string());
        }

        #[test]
        fn test_response_message_ids() {
            assert_eq!(PnLSingle::RESPONSE_MESSAGE_IDS, &[IncomingMessages::PnLSingle]);
        }
    }

    mod position_update_tests {
        use super::*;

        #[test]
        fn test_decode_position() {
            // Format: message_type\0version\0account\0contract_id\0symbol\0sec_type\0last_trade_date\0strike\0right\0multiplier\0exchange\0currency\0local_symbol\0trading_class\0position\0avg_cost\0
            let mut message = ResponseMessage::from("61\03\0DU1234567\012345\0AAPL\0STK\0\00.0\0\0\0NASDAQ\0USD\0AAPL\0NMS\0100\050.25\0");

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
            // Format: message_type\0version\0
            let mut message = ResponseMessage::from("62\01\0");

            let result = PositionUpdate::decode(&test_context(), &mut message).unwrap();

            assert!(matches!(result, PositionUpdate::PositionEnd));
        }

        #[test]
        fn test_cancel_message() {
            let request = PositionUpdate::cancel_message(TEST_SERVER_VERSION, None, None).unwrap();

            assert_eq!(request[0], OutgoingMessages::CancelPositions.to_string());
            assert_eq!(request[1], "1");
        }

        #[test]
        fn test_response_message_ids() {
            assert_eq!(
                PositionUpdate::RESPONSE_MESSAGE_IDS,
                &[IncomingMessages::Position, IncomingMessages::PositionEnd]
            );
        }
    }

    mod position_update_multi_tests {
        use super::*;

        #[test]
        fn test_decode_position_multi() {
            // Format: message_type\0version\0request_id\0account\0contract_id\0symbol\0sec_type\0last_trade_date\0strike\0right\0multiplier\0exchange\0currency\0local_symbol\0trading_class\0position\0avg_cost\0model_code\0
            let mut message =
                ResponseMessage::from("71\01\0123\0DU1234567\012345\0AAPL\0STK\0\00.0\0\0\0NASDAQ\0USD\0AAPL\0NMS\0100\050.25\0TARGET2024\0");

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
            // Format: message_type\0version\0request_id\0
            let mut message = ResponseMessage::from("72\01\0123\0");

            let result = PositionUpdateMulti::decode(&test_context(), &mut message).unwrap();

            assert!(matches!(result, PositionUpdateMulti::PositionEnd));
        }

        #[test]
        fn test_cancel_message() {
            let request = PositionUpdateMulti::cancel_message(TEST_SERVER_VERSION, Some(TEST_REQUEST_ID), None).unwrap();

            assert_eq!(request[0], OutgoingMessages::CancelPositionsMulti.to_string());
            assert_eq!(request[1], "1"); // version
            assert_eq!(request[2], TEST_REQUEST_ID.to_string());
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
                &[IncomingMessages::PositionMulti, IncomingMessages::PositionMultiEnd]
            );
        }
    }

    mod account_update_tests {
        use super::*;

        #[test]
        fn test_decode_account_value() {
            // Format: message_type\0version\0key\0value\0currency\0account\0
            let mut message = ResponseMessage::from("6\02\0NetLiquidation\0123456.78\0USD\0DU1234567\0");

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
            // Format: message_type\0version\0contract_id\0symbol\0sec_type\0last_trade_date\0strike\0right\0multiplier\0primary_exchange\0currency\0local_symbol\0trading_class\0position\0market_price\0market_value\0avg_cost\0unrealized_pnl\0realized_pnl\0account\0
            let mut message = ResponseMessage::from(
                "7\08\012345\0AAPL\0STK\020230101\0150.0\0\0\0NASDAQ\0USD\0AAPL\0NMS\0100\0155.0\015500.0\0150.0\0500.0\00.0\0DU1234567\0",
            );

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
            // Format: message_type\0version\0timestamp\0
            let mut message = ResponseMessage::from("8\01\014:30:00\0");

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
            // Format: message_type\0version\0account\0
            let mut message = ResponseMessage::from("54\01\0DU1234567\0");

            let result = AccountUpdate::decode(&test_context(), &mut message).unwrap();

            assert!(matches!(result, AccountUpdate::End));
        }

        #[test]
        fn test_cancel_message() {
            let request = AccountUpdate::cancel_message(TEST_SERVER_VERSION, None, None).unwrap();

            assert_eq!(request[0], OutgoingMessages::RequestAccountData.to_string());
        }

        #[test]
        fn test_response_message_ids() {
            assert_eq!(
                AccountUpdate::RESPONSE_MESSAGE_IDS,
                &[
                    IncomingMessages::AccountValue,
                    IncomingMessages::PortfolioValue,
                    IncomingMessages::AccountUpdateTime,
                    IncomingMessages::AccountDownloadEnd
                ]
            );
        }
    }

    mod account_update_multi_tests {
        use super::*;

        #[test]
        fn test_decode_account_multi_value() {
            // Format: message_type\0version\0request_id\0account\0model_code\0key\0value\0currency\0
            let mut message = ResponseMessage::from("73\01\0123\0DU1234567\0TARGET2024\0NetLiquidation\0123456.78\0USD\0");

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
            // Format: message_type\0version\0request_id\0
            let mut message = ResponseMessage::from("74\01\0123\0");

            let result = AccountUpdateMulti::decode(&test_context(), &mut message).unwrap();

            assert!(matches!(result, AccountUpdateMulti::End));
        }

        #[test]
        fn test_cancel_message() {
            let request = AccountUpdateMulti::cancel_message(TEST_SERVER_VERSION, Some(TEST_REQUEST_ID), None).unwrap();

            assert_eq!(request[0], OutgoingMessages::CancelAccountUpdatesMulti.to_string());
            assert_eq!(request[1], "1"); // version
            assert_eq!(request[2], TEST_REQUEST_ID.to_string());
        }

        #[test]
        fn test_cancel_message_no_request_id() {
            let result = AccountUpdateMulti::cancel_message(TEST_SERVER_VERSION, None, None);

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), Error::Simple(_)));
        }

        #[test]
        fn test_response_message_ids() {
            assert_eq!(
                AccountUpdateMulti::RESPONSE_MESSAGE_IDS,
                &[IncomingMessages::AccountUpdateMulti, IncomingMessages::AccountUpdateMultiEnd]
            );
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
            assert_eq!(result1[0], result2[0]);
            assert_eq!(result1[1], result2[1]);
        }
    }

    // Integration tests with actual decoder functions
    mod integration_tests {
        use super::*;

        #[test]
        fn test_full_account_summary_flow() {
            // Test decoding a series of account summary messages
            let messages = vec![
                ResponseMessage::from("63\01\0123\0DU1234567\0NetLiquidation\0100000.00\0USD\0"),
                ResponseMessage::from("63\01\0123\0DU1234567\0TotalCashValue\050000.00\0USD\0"),
                ResponseMessage::from("64\01\0123\0"),
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
            // Test decoding a series of position messages
            let messages = vec![
                ResponseMessage::from("61\03\0DU1234567\012345\0AAPL\0STK\0\00.0\0\0\0NASDAQ\0USD\0AAPL\0NMS\0100\050.25\0"),
                ResponseMessage::from("61\03\0DU7654321\067890\0GOOGL\0STK\0\00.0\0\0\0NASDAQ\0USD\0GOOGL\0NMS\0200\075.50\0"),
                ResponseMessage::from("62\01\0"),
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
}
