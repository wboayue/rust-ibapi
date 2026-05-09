//! Builders for contracts-domain request and response messages.

use super::{RequestEncoder, ResponseProtoEncoder};
use crate::common::test_utils::helpers::constants::TEST_REQ_ID_FIRST;
use crate::contracts::{Contract, SecurityType};
use crate::messages::OutgoingMessages;
use crate::proto;
use crate::proto::encoders::{encode_contract, some_str};

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

#[derive(Clone, Copy, Debug, Default)]
pub struct MarketRuleRequestBuilder {
    pub market_rule_id: i32,
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
single_req_id_request_builder!(CancelContractDataRequestBuilder, CancelContractData, OutgoingMessages::CancelContractData);

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
// Response builders
// =============================================================================

/// Builder for `ContractData` (msg 10) responses. Mirrors `proto::ContractData`
/// (req_id + Contract + ContractDetails). Only the fields exercised by tests
/// have setters; everything else stays at the proto default.
#[derive(Clone, Debug)]
pub struct ContractDataResponse {
    pub request_id: i32,
    pub contract_id: i32,
    pub symbol: String,
    pub security_type: String,
    pub last_trade_date_or_contract_month: String,
    pub multiplier: String,
    pub exchange: String,
    pub primary_exchange: String,
    pub currency: String,
    pub local_symbol: String,
    pub trading_class: String,
    pub market_name: String,
    pub min_tick: String,
    pub order_types: String,
    pub valid_exchanges: String,
    pub long_name: String,
    pub industry: String,
    pub category: String,
    pub subcategory: String,
    pub time_zone_id: String,
    pub stock_type: String,
    /// Default `"1"` is load-bearing — `test_contract_details` validators assert `min_size == 1.0`.
    pub min_size: String,
    /// Default `"1"` is load-bearing — see `min_size`.
    pub size_increment: String,
    /// Default `"100"` is load-bearing — see `min_size`.
    pub suggested_size_increment: String,
}

impl Default for ContractDataResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            contract_id: 0,
            symbol: String::new(),
            security_type: String::new(),
            last_trade_date_or_contract_month: String::new(),
            multiplier: String::new(),
            exchange: String::new(),
            primary_exchange: String::new(),
            currency: String::new(),
            local_symbol: String::new(),
            trading_class: String::new(),
            market_name: String::new(),
            min_tick: "0.01".to_string(),
            order_types: String::new(),
            valid_exchanges: String::new(),
            long_name: String::new(),
            industry: String::new(),
            category: String::new(),
            subcategory: String::new(),
            time_zone_id: String::new(),
            stock_type: String::new(),
            min_size: "1".to_string(),
            size_increment: "1".to_string(),
            suggested_size_increment: "100".to_string(),
        }
    }
}

impl ContractDataResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
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
    pub fn multiplier(mut self, v: impl Into<String>) -> Self {
        self.multiplier = v.into();
        self
    }
    pub fn exchange(mut self, v: impl Into<String>) -> Self {
        self.exchange = v.into();
        self
    }
    pub fn primary_exchange(mut self, v: impl Into<String>) -> Self {
        self.primary_exchange = v.into();
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
    pub fn market_name(mut self, v: impl Into<String>) -> Self {
        self.market_name = v.into();
        self
    }
    pub fn min_tick(mut self, v: impl Into<String>) -> Self {
        self.min_tick = v.into();
        self
    }
    pub fn order_types(mut self, v: impl Into<String>) -> Self {
        self.order_types = v.into();
        self
    }
    pub fn valid_exchanges(mut self, v: impl Into<String>) -> Self {
        self.valid_exchanges = v.into();
        self
    }
    pub fn long_name(mut self, v: impl Into<String>) -> Self {
        self.long_name = v.into();
        self
    }
    pub fn industry(mut self, v: impl Into<String>) -> Self {
        self.industry = v.into();
        self
    }
    pub fn category(mut self, v: impl Into<String>) -> Self {
        self.category = v.into();
        self
    }
    pub fn subcategory(mut self, v: impl Into<String>) -> Self {
        self.subcategory = v.into();
        self
    }
    pub fn time_zone_id(mut self, v: impl Into<String>) -> Self {
        self.time_zone_id = v.into();
        self
    }
    pub fn stock_type(mut self, v: impl Into<String>) -> Self {
        self.stock_type = v.into();
        self
    }
}

impl ResponseProtoEncoder for ContractDataResponse {
    type Proto = proto::ContractData;

    fn to_proto(&self) -> Self::Proto {
        proto::ContractData {
            req_id: Some(self.request_id),
            contract: Some(proto::Contract {
                con_id: Some(self.contract_id),
                symbol: some_str(&self.symbol),
                sec_type: some_str(&self.security_type),
                last_trade_date_or_contract_month: some_str(&self.last_trade_date_or_contract_month),
                multiplier: if self.multiplier.is_empty() {
                    None
                } else {
                    self.multiplier.parse().ok()
                },
                exchange: some_str(&self.exchange),
                primary_exch: some_str(&self.primary_exchange),
                currency: some_str(&self.currency),
                local_symbol: some_str(&self.local_symbol),
                trading_class: some_str(&self.trading_class),
                ..Default::default()
            }),
            contract_details: Some(proto::ContractDetails {
                market_name: some_str(&self.market_name),
                min_tick: some_str(&self.min_tick),
                order_types: some_str(&self.order_types),
                valid_exchanges: some_str(&self.valid_exchanges),
                long_name: some_str(&self.long_name),
                industry: some_str(&self.industry),
                category: some_str(&self.category),
                subcategory: some_str(&self.subcategory),
                time_zone_id: some_str(&self.time_zone_id),
                stock_type: some_str(&self.stock_type),
                min_size: some_str(&self.min_size),
                size_increment: some_str(&self.size_increment),
                suggested_size_increment: some_str(&self.suggested_size_increment),
                ..Default::default()
            }),
        }
    }
}

/// Builder for `SymbolSamples` (msg 79) responses.
#[derive(Clone, Debug)]
pub struct SymbolSamplesEntry {
    pub contract_id: i32,
    pub symbol: String,
    pub security_type: String,
    pub primary_exchange: String,
    pub currency: String,
    pub description: String,
    pub derivative_security_types: Vec<String>,
}

impl SymbolSamplesEntry {
    pub fn primary_exchange(mut self, v: impl Into<String>) -> Self {
        self.primary_exchange = v.into();
        self
    }
    pub fn description(mut self, v: impl Into<String>) -> Self {
        self.description = v.into();
        self
    }
    pub fn derivative_security_types(mut self, v: Vec<String>) -> Self {
        self.derivative_security_types = v;
        self
    }
}

#[derive(Clone, Debug, Default)]
pub struct SymbolSamplesResponse {
    pub request_id: i32,
    pub entries: Vec<SymbolSamplesEntry>,
}

impl SymbolSamplesResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn entry(mut self, e: SymbolSamplesEntry) -> Self {
        self.entries.push(e);
        self
    }
}

impl ResponseProtoEncoder for SymbolSamplesResponse {
    type Proto = proto::SymbolSamples;

    fn to_proto(&self) -> Self::Proto {
        proto::SymbolSamples {
            req_id: Some(self.request_id),
            contract_descriptions: self
                .entries
                .iter()
                .map(|e| proto::ContractDescription {
                    contract: Some(proto::Contract {
                        con_id: Some(e.contract_id),
                        symbol: some_str(&e.symbol),
                        sec_type: some_str(&e.security_type),
                        primary_exch: some_str(&e.primary_exchange),
                        currency: some_str(&e.currency),
                        description: some_str(&e.description),
                        ..Default::default()
                    }),
                    derivative_sec_types: e.derivative_security_types.clone(),
                })
                .collect(),
        }
    }
}

/// Builder for `MarketRule` (msg 87) responses.
#[derive(Clone, Debug)]
pub struct MarketRuleResponse {
    pub market_rule_id: i32,
    pub price_increments: Vec<(f64, f64)>,
}

impl MarketRuleResponse {
    pub fn increment(mut self, low_edge: f64, increment: f64) -> Self {
        self.price_increments.push((low_edge, increment));
        self
    }
}

impl ResponseProtoEncoder for MarketRuleResponse {
    type Proto = proto::MarketRule;

    fn to_proto(&self) -> Self::Proto {
        proto::MarketRule {
            market_rule_id: Some(self.market_rule_id),
            price_increments: self
                .price_increments
                .iter()
                .map(|(low_edge, increment)| proto::PriceIncrement {
                    low_edge: Some(*low_edge),
                    increment: Some(*increment),
                })
                .collect(),
        }
    }
}

/// Builder for `SecurityDefinitionOptionParameter` (msg 75) responses.
#[derive(Clone, Debug)]
pub struct OptionChainResponse {
    pub request_id: i32,
    pub exchange: String,
    pub underlying_contract_id: i32,
    pub trading_class: String,
    pub multiplier: String,
    pub expirations: Vec<String>,
    pub strikes: Vec<f64>,
}

impl Default for OptionChainResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            exchange: String::new(),
            underlying_contract_id: 0,
            trading_class: String::new(),
            multiplier: "100".to_string(),
            expirations: Vec::new(),
            strikes: Vec::new(),
        }
    }
}

impl OptionChainResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn exchange(mut self, v: impl Into<String>) -> Self {
        self.exchange = v.into();
        self
    }
    pub fn underlying_contract_id(mut self, v: i32) -> Self {
        self.underlying_contract_id = v;
        self
    }
    pub fn trading_class(mut self, v: impl Into<String>) -> Self {
        self.trading_class = v.into();
        self
    }
    pub fn multiplier(mut self, v: impl Into<String>) -> Self {
        self.multiplier = v.into();
        self
    }
    pub fn expirations(mut self, v: Vec<String>) -> Self {
        self.expirations = v;
        self
    }
    pub fn strikes(mut self, v: Vec<f64>) -> Self {
        self.strikes = v;
        self
    }
}

impl ResponseProtoEncoder for OptionChainResponse {
    type Proto = proto::SecDefOptParameter;

    fn to_proto(&self) -> Self::Proto {
        proto::SecDefOptParameter {
            req_id: Some(self.request_id),
            exchange: some_str(&self.exchange),
            underlying_con_id: Some(self.underlying_contract_id),
            trading_class: some_str(&self.trading_class),
            multiplier: some_str(&self.multiplier),
            expirations: self.expirations.clone(),
            strikes: self.strikes.clone(),
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

pub fn contract_data() -> ContractDataResponse {
    ContractDataResponse::default()
}

pub fn symbol_samples() -> SymbolSamplesResponse {
    SymbolSamplesResponse::default()
}

pub fn symbol_samples_entry(contract_id: i32, symbol: impl Into<String>) -> SymbolSamplesEntry {
    SymbolSamplesEntry {
        contract_id,
        symbol: symbol.into(),
        security_type: "STK".to_string(),
        primary_exchange: "NASDAQ".to_string(),
        currency: "USD".to_string(),
        description: String::new(),
        derivative_security_types: Vec::new(),
    }
}

pub fn market_rule(market_rule_id: i32) -> MarketRuleResponse {
    MarketRuleResponse {
        market_rule_id,
        price_increments: Vec::new(),
    }
}

pub fn option_chain() -> OptionChainResponse {
    OptionChainResponse::default()
}
