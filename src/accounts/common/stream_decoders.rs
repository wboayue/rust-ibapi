//! Common DataStream implementations for accounts module
//!
//! This module contains the DataStream trait implementations that are shared
//! between sync and async versions, avoiding code duplication.

use crate::accounts::*;
use crate::messages::{IncomingMessages, RequestMessage, ResponseMessage};
use crate::subscriptions::{ResponseContext, StreamDecoder};
use crate::Error;

use super::{decoders, encoders};
use crate::common::error_helpers;

impl StreamDecoder<AccountSummaryResult> for AccountSummaryResult {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::AccountSummary, IncomingMessages::AccountSummaryEnd];

    fn decode(server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::AccountSummary => Ok(AccountSummaryResult::Summary(decoders::decode_account_summary(server_version, message)?)),
            IncomingMessages::AccountSummaryEnd => Ok(AccountSummaryResult::End),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&ResponseContext>) -> Result<RequestMessage, Error> {
        let request_id = error_helpers::require_request_id(request_id)?;
        encoders::encode_cancel_account_summary(request_id)
    }
}

impl StreamDecoder<PnL> for PnL {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::PnL];

    fn decode(server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_pnl(server_version, message)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&ResponseContext>) -> Result<RequestMessage, Error> {
        let request_id = error_helpers::require_request_id(request_id)?;
        encoders::encode_cancel_pnl(request_id)
    }
}

impl StreamDecoder<PnLSingle> for PnLSingle {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::PnLSingle];

    fn decode(server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        decoders::decode_pnl_single(server_version, message)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&ResponseContext>) -> Result<RequestMessage, Error> {
        let request_id = error_helpers::require_request_id(request_id)?;
        encoders::encode_cancel_pnl_single(request_id)
    }
}

impl StreamDecoder<PositionUpdate> for PositionUpdate {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::Position, IncomingMessages::PositionEnd];

    fn decode(_server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::Position => Ok(PositionUpdate::Position(decoders::decode_position(message)?)),
            IncomingMessages::PositionEnd => Ok(PositionUpdate::PositionEnd),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(_server_version: i32, _request_id: Option<i32>, _context: Option<&ResponseContext>) -> Result<RequestMessage, Error> {
        encoders::encode_cancel_positions()
    }
}

impl StreamDecoder<PositionUpdateMulti> for PositionUpdateMulti {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::PositionMulti, IncomingMessages::PositionMultiEnd];

    fn decode(_server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::PositionMulti => Ok(PositionUpdateMulti::Position(decoders::decode_position_multi(message)?)),
            IncomingMessages::PositionMultiEnd => Ok(PositionUpdateMulti::PositionEnd),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&ResponseContext>) -> Result<RequestMessage, Error> {
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

    fn decode(server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::AccountValue => Ok(AccountUpdate::AccountValue(decoders::decode_account_value(message)?)),
            IncomingMessages::PortfolioValue => Ok(AccountUpdate::PortfolioValue(decoders::decode_account_portfolio_value(
                server_version,
                message,
            )?)),
            IncomingMessages::AccountUpdateTime => Ok(AccountUpdate::UpdateTime(decoders::decode_account_update_time(message)?)),
            IncomingMessages::AccountDownloadEnd => Ok(AccountUpdate::End),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(server_version: i32, _request_id: Option<i32>, _context: Option<&ResponseContext>) -> Result<RequestMessage, Error> {
        encoders::encode_cancel_account_updates(server_version)
    }
}

impl StreamDecoder<AccountUpdateMulti> for AccountUpdateMulti {
    const RESPONSE_MESSAGE_IDS: &'static [IncomingMessages] = &[IncomingMessages::AccountUpdateMulti, IncomingMessages::AccountUpdateMultiEnd];

    fn decode(_server_version: i32, message: &mut ResponseMessage) -> Result<Self, Error> {
        match message.message_type() {
            IncomingMessages::AccountUpdateMulti => Ok(AccountUpdateMulti::AccountMultiValue(decoders::decode_account_multi_value(message)?)),
            IncomingMessages::AccountUpdateMultiEnd => Ok(AccountUpdateMulti::End),
            message => Err(Error::Simple(format!("unexpected message: {message:?}"))),
        }
    }

    fn cancel_message(server_version: i32, request_id: Option<i32>, _context: Option<&ResponseContext>) -> Result<RequestMessage, Error> {
        let request_id = error_helpers::require_request_id_for(request_id, "encode cancel account updates multi")?;
        encoders::encode_cancel_account_updates_multi(server_version, request_id)
    }
}
