//! Builders for contracts-domain request messages.
//!
//! No response builders here: the migrated sync/async tests reuse the inline
//! pipe literals in `contracts/common/test_tables.rs`, and the *End sentinels
//! contain only a request_id (no domain payload), so a typed builder would
//! only re-test prost — see PR 3 §"Lessons learned". Add a response builder
//! when a real test needs to construct a non-trivial body.

use super::RequestEncoder;
use crate::common::test_utils::helpers::constants::TEST_REQ_ID_FIRST;
use crate::contracts::{Contract, SecurityType};
use crate::messages::OutgoingMessages;
use crate::proto;
use crate::proto::encoders::encode_contract;

// =============================================================================
// Request builders
// =============================================================================

#[derive(Clone, Debug)]
pub struct ContractDataRequestBuilder {
    pub request_id: i32,
    pub contract: Contract,
}

impl Default for ContractDataRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            contract: Contract::default(),
        }
    }
}

impl ContractDataRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn contract(mut self, contract: &Contract) -> Self {
        self.contract = contract.clone();
        self
    }
}

impl RequestEncoder for ContractDataRequestBuilder {
    type Proto = proto::ContractDataRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestContractData;

    fn to_proto(&self) -> Self::Proto {
        proto::ContractDataRequest {
            req_id: Some(self.request_id),
            contract: Some(encode_contract(&self.contract)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MatchingSymbolsRequestBuilder {
    pub request_id: i32,
    pub pattern: String,
}

impl Default for MatchingSymbolsRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            pattern: String::new(),
        }
    }
}

impl MatchingSymbolsRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn pattern(mut self, v: impl Into<String>) -> Self {
        self.pattern = v.into();
        self
    }
}

impl RequestEncoder for MatchingSymbolsRequestBuilder {
    type Proto = proto::MatchingSymbolsRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestMatchingSymbols;

    fn to_proto(&self) -> Self::Proto {
        proto::MatchingSymbolsRequest {
            req_id: Some(self.request_id),
            pattern: Some(self.pattern.clone()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MarketRuleRequestBuilder {
    pub market_rule_id: i32,
}

impl Default for MarketRuleRequestBuilder {
    fn default() -> Self {
        Self { market_rule_id: 26 }
    }
}

impl MarketRuleRequestBuilder {
    pub fn market_rule_id(mut self, v: i32) -> Self {
        self.market_rule_id = v;
        self
    }
}

impl RequestEncoder for MarketRuleRequestBuilder {
    type Proto = proto::MarketRuleRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestMarketRule;

    fn to_proto(&self) -> Self::Proto {
        proto::MarketRuleRequest {
            market_rule_id: Some(self.market_rule_id),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CalculateOptionPriceRequestBuilder {
    pub request_id: i32,
    pub contract: Contract,
    pub volatility: f64,
    pub underlying_price: f64,
}

impl Default for CalculateOptionPriceRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            contract: Contract::default(),
            volatility: 0.0,
            underlying_price: 0.0,
        }
    }
}

impl CalculateOptionPriceRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn contract(mut self, contract: &Contract) -> Self {
        self.contract = contract.clone();
        self
    }
    pub fn volatility(mut self, v: f64) -> Self {
        self.volatility = v;
        self
    }
    pub fn underlying_price(mut self, v: f64) -> Self {
        self.underlying_price = v;
        self
    }
}

impl RequestEncoder for CalculateOptionPriceRequestBuilder {
    type Proto = proto::CalculateOptionPriceRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::ReqCalcOptionPrice;

    fn to_proto(&self) -> Self::Proto {
        proto::CalculateOptionPriceRequest {
            req_id: Some(self.request_id),
            contract: Some(encode_contract(&self.contract)),
            volatility: Some(self.volatility),
            under_price: Some(self.underlying_price),
            option_price_options: Default::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CalculateImpliedVolatilityRequestBuilder {
    pub request_id: i32,
    pub contract: Contract,
    pub option_price: f64,
    pub underlying_price: f64,
}

impl Default for CalculateImpliedVolatilityRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            contract: Contract::default(),
            option_price: 0.0,
            underlying_price: 0.0,
        }
    }
}

impl CalculateImpliedVolatilityRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn contract(mut self, contract: &Contract) -> Self {
        self.contract = contract.clone();
        self
    }
    pub fn option_price(mut self, v: f64) -> Self {
        self.option_price = v;
        self
    }
    pub fn underlying_price(mut self, v: f64) -> Self {
        self.underlying_price = v;
        self
    }
}

impl RequestEncoder for CalculateImpliedVolatilityRequestBuilder {
    type Proto = proto::CalculateImpliedVolatilityRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::ReqCalcImpliedVolat;

    fn to_proto(&self) -> Self::Proto {
        proto::CalculateImpliedVolatilityRequest {
            req_id: Some(self.request_id),
            contract: Some(encode_contract(&self.contract)),
            option_price: Some(self.option_price),
            under_price: Some(self.underlying_price),
            implied_volatility_options: Default::default(),
        }
    }
}

// CancelOptionPrice / CancelImpliedVolatility builders intentionally omitted:
// the production cancel path goes through `OptionComputation::cancel_message`
// in `stream_decoders`, which is exercised end-to-end by `test_cancel_messages`.
cancel_by_request_id_builder!(CancelContractDataRequestBuilder, CancelContractData, OutgoingMessages::CancelContractData);

#[derive(Clone, Debug)]
pub struct OptionChainRequestBuilder {
    pub request_id: i32,
    pub symbol: String,
    pub exchange: String,
    pub security_type: SecurityType,
    pub contract_id: i32,
}

impl Default for OptionChainRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            symbol: String::new(),
            exchange: String::new(),
            security_type: SecurityType::Stock,
            contract_id: 0,
        }
    }
}

impl OptionChainRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn symbol(mut self, v: impl Into<String>) -> Self {
        self.symbol = v.into();
        self
    }
    pub fn exchange(mut self, v: impl Into<String>) -> Self {
        self.exchange = v.into();
        self
    }
    pub fn security_type(mut self, v: SecurityType) -> Self {
        self.security_type = v;
        self
    }
    pub fn contract_id(mut self, v: i32) -> Self {
        self.contract_id = v;
        self
    }
}

impl RequestEncoder for OptionChainRequestBuilder {
    type Proto = proto::SecDefOptParamsRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestSecurityDefinitionOptionalParameters;

    fn to_proto(&self) -> Self::Proto {
        proto::SecDefOptParamsRequest {
            req_id: Some(self.request_id),
            underlying_symbol: Some(self.symbol.clone()),
            fut_fop_exchange: Some(self.exchange.clone()),
            underlying_sec_type: Some(self.security_type.to_string()),
            underlying_con_id: Some(self.contract_id),
        }
    }
}

// =============================================================================
// Entry-point functions
// =============================================================================

pub fn contract_data_request() -> ContractDataRequestBuilder {
    ContractDataRequestBuilder::default()
}

pub fn matching_symbols_request() -> MatchingSymbolsRequestBuilder {
    MatchingSymbolsRequestBuilder::default()
}

pub fn market_rule_request() -> MarketRuleRequestBuilder {
    MarketRuleRequestBuilder::default()
}

pub fn calculate_option_price_request() -> CalculateOptionPriceRequestBuilder {
    CalculateOptionPriceRequestBuilder::default()
}

pub fn calculate_implied_volatility_request() -> CalculateImpliedVolatilityRequestBuilder {
    CalculateImpliedVolatilityRequestBuilder::default()
}

pub fn cancel_contract_data_request() -> CancelContractDataRequestBuilder {
    CancelContractDataRequestBuilder::default()
}

pub fn option_chain_request() -> OptionChainRequestBuilder {
    OptionChainRequestBuilder::default()
}
