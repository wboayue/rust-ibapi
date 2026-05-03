//! Builders for position-related response messages.
//!
//! Produces wire-format payloads for `Position` (61), `PositionEnd` (62),
//! `PositionMulti` (71), and `PositionMultiEnd` (72) at the message version
//! consumed by the current decoders.

use super::{RequestEncoder, ResponseEncoder};
use crate::common::test_utils::helpers::constants::{TEST_ACCOUNT, TEST_CONTRACT_ID, TEST_TICKER_ID};
use crate::messages::OutgoingMessages;
use crate::proto;
use crate::proto::encoders::{some_f64_ne, some_i32_ne, some_str};
use prost::Message;

const POSITION_VERSION: i32 = 3;
const POSITION_END_VERSION: i32 = 1;
const POSITION_MULTI_VERSION: i32 = 3;
const POSITION_MULTI_END_VERSION: i32 = 1;

#[derive(Clone, Debug)]
pub struct PositionResponse {
    pub account: String,
    pub contract_id: i32,
    pub symbol: String,
    pub security_type: String,
    pub last_trade_date_or_contract_month: String,
    pub strike: f64,
    pub right: String,
    pub multiplier: String,
    pub exchange: String,
    pub currency: String,
    pub local_symbol: String,
    pub trading_class: String,
    pub position: f64,
    pub average_cost: f64,
}

impl Default for PositionResponse {
    fn default() -> Self {
        Self {
            account: TEST_ACCOUNT.to_string(),
            contract_id: TEST_CONTRACT_ID,
            symbol: "TSLA".to_string(),
            security_type: "STK".to_string(),
            last_trade_date_or_contract_month: String::new(),
            strike: 0.0,
            right: String::new(),
            multiplier: String::new(),
            exchange: "NASDAQ".to_string(),
            currency: "USD".to_string(),
            local_symbol: "TSLA".to_string(),
            trading_class: "NMS".to_string(),
            position: 500.0,
            average_cost: 196.77,
        }
    }
}

impl PositionResponse {
    pub fn account(mut self, v: impl Into<String>) -> Self {
        self.account = v.into();
        self
    }
    pub fn contract_id(mut self, v: i32) -> Self {
        self.contract_id = v;
        self
    }
    pub fn symbol(mut self, v: impl Into<String>) -> Self {
        self.symbol = v.into();
        self
    }
    pub fn security_type(mut self, v: impl Into<String>) -> Self {
        self.security_type = v.into();
        self
    }
    pub fn last_trade_date_or_contract_month(mut self, v: impl Into<String>) -> Self {
        self.last_trade_date_or_contract_month = v.into();
        self
    }
    pub fn strike(mut self, v: f64) -> Self {
        self.strike = v;
        self
    }
    pub fn right(mut self, v: impl Into<String>) -> Self {
        self.right = v.into();
        self
    }
    pub fn multiplier(mut self, v: impl Into<String>) -> Self {
        self.multiplier = v.into();
        self
    }
    pub fn exchange(mut self, v: impl Into<String>) -> Self {
        self.exchange = v.into();
        self
    }
    pub fn currency(mut self, v: impl Into<String>) -> Self {
        self.currency = v.into();
        self
    }
    pub fn local_symbol(mut self, v: impl Into<String>) -> Self {
        self.local_symbol = v.into();
        self
    }
    pub fn trading_class(mut self, v: impl Into<String>) -> Self {
        self.trading_class = v.into();
        self
    }
    pub fn position(mut self, v: f64) -> Self {
        self.position = v;
        self
    }
    pub fn average_cost(mut self, v: f64) -> Self {
        self.average_cost = v;
        self
    }
}

impl ResponseEncoder for PositionResponse {
    fn fields(&self) -> Vec<String> {
        vec![
            "61".to_string(),
            POSITION_VERSION.to_string(),
            self.account.clone(),
            self.contract_id.to_string(),
            self.symbol.clone(),
            self.security_type.clone(),
            self.last_trade_date_or_contract_month.clone(),
            self.strike.to_string(),
            self.right.clone(),
            self.multiplier.clone(),
            self.exchange.clone(),
            self.currency.clone(),
            self.local_symbol.clone(),
            self.trading_class.clone(),
            self.position.to_string(),
            self.average_cost.to_string(),
        ]
    }
}

impl PositionResponse {
    pub fn to_proto(&self) -> proto::Position {
        proto::Position {
            account: Some(self.account.clone()),
            contract: Some(proto::Contract {
                con_id: some_i32_ne(self.contract_id, 0),
                symbol: some_str(&self.symbol),
                sec_type: some_str(&self.security_type),
                last_trade_date_or_contract_month: some_str(&self.last_trade_date_or_contract_month),
                strike: some_f64_ne(self.strike, 0.0),
                right: some_str(&self.right),
                multiplier: self.multiplier.parse::<f64>().ok(),
                exchange: some_str(&self.exchange),
                currency: some_str(&self.currency),
                local_symbol: some_str(&self.local_symbol),
                trading_class: some_str(&self.trading_class),
                ..Default::default()
            }),
            position: Some(self.position.to_string()),
            avg_cost: Some(self.average_cost),
        }
    }

    pub fn encode_proto(&self) -> Vec<u8> {
        self.to_proto().encode_to_vec()
    }
}

#[derive(Clone, Debug, Default)]
pub struct PositionEndResponse;

impl ResponseEncoder for PositionEndResponse {
    fn fields(&self) -> Vec<String> {
        vec!["62".to_string(), POSITION_END_VERSION.to_string()]
    }
}

impl PositionEndResponse {
    pub fn to_proto(&self) -> proto::PositionEnd {
        proto::PositionEnd {}
    }

    pub fn encode_proto(&self) -> Vec<u8> {
        self.to_proto().encode_to_vec()
    }
}

#[derive(Clone, Debug)]
pub struct PositionMultiResponse {
    pub request_id: i32,
    pub account: String,
    pub contract_id: i32,
    pub symbol: String,
    pub security_type: String,
    pub last_trade_date_or_contract_month: String,
    pub strike: f64,
    pub right: String,
    pub multiplier: String,
    pub exchange: String,
    pub currency: String,
    pub local_symbol: String,
    pub trading_class: String,
    pub position: f64,
    pub average_cost: f64,
    pub model_code: String,
}

impl Default for PositionMultiResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_TICKER_ID,
            account: TEST_ACCOUNT.to_string(),
            contract_id: TEST_CONTRACT_ID,
            symbol: "TSLA".to_string(),
            security_type: "STK".to_string(),
            last_trade_date_or_contract_month: String::new(),
            strike: 0.0,
            right: String::new(),
            multiplier: String::new(),
            exchange: "NASDAQ".to_string(),
            currency: "USD".to_string(),
            local_symbol: "TSLA".to_string(),
            trading_class: "NMS".to_string(),
            position: 500.0,
            average_cost: 196.77,
            model_code: String::new(),
        }
    }
}

impl PositionMultiResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn account(mut self, v: impl Into<String>) -> Self {
        self.account = v.into();
        self
    }
    pub fn contract_id(mut self, v: i32) -> Self {
        self.contract_id = v;
        self
    }
    pub fn symbol(mut self, v: impl Into<String>) -> Self {
        self.symbol = v.into();
        self
    }
    pub fn security_type(mut self, v: impl Into<String>) -> Self {
        self.security_type = v.into();
        self
    }
    pub fn last_trade_date_or_contract_month(mut self, v: impl Into<String>) -> Self {
        self.last_trade_date_or_contract_month = v.into();
        self
    }
    pub fn strike(mut self, v: f64) -> Self {
        self.strike = v;
        self
    }
    pub fn right(mut self, v: impl Into<String>) -> Self {
        self.right = v.into();
        self
    }
    pub fn multiplier(mut self, v: impl Into<String>) -> Self {
        self.multiplier = v.into();
        self
    }
    pub fn exchange(mut self, v: impl Into<String>) -> Self {
        self.exchange = v.into();
        self
    }
    pub fn currency(mut self, v: impl Into<String>) -> Self {
        self.currency = v.into();
        self
    }
    pub fn local_symbol(mut self, v: impl Into<String>) -> Self {
        self.local_symbol = v.into();
        self
    }
    pub fn trading_class(mut self, v: impl Into<String>) -> Self {
        self.trading_class = v.into();
        self
    }
    pub fn position(mut self, v: f64) -> Self {
        self.position = v;
        self
    }
    pub fn average_cost(mut self, v: f64) -> Self {
        self.average_cost = v;
        self
    }
    pub fn model_code(mut self, v: impl Into<String>) -> Self {
        self.model_code = v.into();
        self
    }
}

impl ResponseEncoder for PositionMultiResponse {
    fn fields(&self) -> Vec<String> {
        vec![
            "71".to_string(),
            POSITION_MULTI_VERSION.to_string(),
            self.request_id.to_string(),
            self.account.clone(),
            self.contract_id.to_string(),
            self.symbol.clone(),
            self.security_type.clone(),
            self.last_trade_date_or_contract_month.clone(),
            self.strike.to_string(),
            self.right.clone(),
            self.multiplier.clone(),
            self.exchange.clone(),
            self.currency.clone(),
            self.local_symbol.clone(),
            self.trading_class.clone(),
            self.position.to_string(),
            self.average_cost.to_string(),
            self.model_code.clone(),
        ]
    }
}

impl PositionMultiResponse {
    pub fn to_proto(&self) -> proto::PositionMulti {
        proto::PositionMulti {
            req_id: Some(self.request_id),
            account: Some(self.account.clone()),
            contract: Some(proto::Contract {
                con_id: some_i32_ne(self.contract_id, 0),
                symbol: some_str(&self.symbol),
                sec_type: some_str(&self.security_type),
                last_trade_date_or_contract_month: some_str(&self.last_trade_date_or_contract_month),
                strike: some_f64_ne(self.strike, 0.0),
                right: some_str(&self.right),
                multiplier: self.multiplier.parse::<f64>().ok(),
                exchange: some_str(&self.exchange),
                currency: some_str(&self.currency),
                local_symbol: some_str(&self.local_symbol),
                trading_class: some_str(&self.trading_class),
                ..Default::default()
            }),
            position: Some(self.position.to_string()),
            avg_cost: Some(self.average_cost),
            model_code: some_str(&self.model_code),
        }
    }

    pub fn encode_proto(&self) -> Vec<u8> {
        self.to_proto().encode_to_vec()
    }
}

#[derive(Clone, Debug)]
pub struct PositionMultiEndResponse {
    pub request_id: i32,
}

impl Default for PositionMultiEndResponse {
    fn default() -> Self {
        Self { request_id: TEST_TICKER_ID }
    }
}

impl PositionMultiEndResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
}

impl ResponseEncoder for PositionMultiEndResponse {
    fn fields(&self) -> Vec<String> {
        vec!["72".to_string(), POSITION_MULTI_END_VERSION.to_string(), self.request_id.to_string()]
    }
}

impl PositionMultiEndResponse {
    pub fn to_proto(&self) -> proto::PositionMultiEnd {
        proto::PositionMultiEnd {
            req_id: Some(self.request_id),
        }
    }

    pub fn encode_proto(&self) -> Vec<u8> {
        self.to_proto().encode_to_vec()
    }
}

// === Request builders ===

#[derive(Clone, Copy, Debug, Default)]
pub struct PositionsRequestBuilder;

impl RequestEncoder for PositionsRequestBuilder {
    type Proto = proto::PositionsRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestPositions;

    fn to_proto(&self) -> Self::Proto {
        proto::PositionsRequest {}
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CancelPositionsRequestBuilder;

impl RequestEncoder for CancelPositionsRequestBuilder {
    type Proto = proto::CancelPositions;
    const MSG_ID: OutgoingMessages = OutgoingMessages::CancelPositions;

    fn to_proto(&self) -> Self::Proto {
        proto::CancelPositions {}
    }
}

#[derive(Clone, Debug)]
pub struct PositionsMultiRequestBuilder {
    pub request_id: i32,
    pub account: String,
    pub model_code: Option<String>,
}

impl Default for PositionsMultiRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_TICKER_ID,
            account: TEST_ACCOUNT.to_string(),
            model_code: None,
        }
    }
}

impl PositionsMultiRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn account(mut self, v: impl Into<String>) -> Self {
        self.account = v.into();
        self
    }
    pub fn model_code(mut self, v: impl Into<String>) -> Self {
        self.model_code = Some(v.into());
        self
    }
}

impl RequestEncoder for PositionsMultiRequestBuilder {
    type Proto = proto::PositionsMultiRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestPositionsMulti;

    fn to_proto(&self) -> Self::Proto {
        proto::PositionsMultiRequest {
            req_id: Some(self.request_id),
            account: Some(self.account.clone()),
            model_code: self.model_code.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CancelPositionsMultiRequestBuilder {
    pub request_id: i32,
}

impl Default for CancelPositionsMultiRequestBuilder {
    fn default() -> Self {
        Self { request_id: TEST_TICKER_ID }
    }
}

impl CancelPositionsMultiRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
}

impl RequestEncoder for CancelPositionsMultiRequestBuilder {
    type Proto = proto::CancelPositionsMulti;
    const MSG_ID: OutgoingMessages = OutgoingMessages::CancelPositionsMulti;

    fn to_proto(&self) -> Self::Proto {
        proto::CancelPositionsMulti {
            req_id: Some(self.request_id),
        }
    }
}

pub fn position() -> PositionResponse {
    PositionResponse::default()
}

pub fn position_end() -> PositionEndResponse {
    PositionEndResponse
}

pub fn position_multi() -> PositionMultiResponse {
    PositionMultiResponse::default()
}

pub fn position_multi_end() -> PositionMultiEndResponse {
    PositionMultiEndResponse::default()
}

pub fn request_positions() -> PositionsRequestBuilder {
    PositionsRequestBuilder
}

pub fn cancel_positions() -> CancelPositionsRequestBuilder {
    CancelPositionsRequestBuilder
}

pub fn request_positions_multi() -> PositionsMultiRequestBuilder {
    PositionsMultiRequestBuilder::default()
}

pub fn cancel_positions_multi() -> CancelPositionsMultiRequestBuilder {
    CancelPositionsMultiRequestBuilder::default()
}

#[cfg(test)]
#[path = "positions_tests.rs"]
mod tests;
