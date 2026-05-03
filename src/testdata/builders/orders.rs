//! Builders for orders-domain response and request messages.

use super::{RequestEncoder, ResponseEncoder, ResponseProtoEncoder};
use crate::common::test_utils::helpers::constants::{TEST_ACCOUNT, TEST_REQ_ID_FIRST};
use crate::contracts::Contract;
use crate::messages::OutgoingMessages;
use crate::orders::{ExecutionFilter, ExerciseAction, Order};
use crate::proto;
use crate::proto::encoders::{
    encode_contract, encode_contract_with_order, encode_execution_filter, encode_order, encode_order_cancel, some_bool, some_str,
};

const TEST_PERM_ID: i64 = 1376327563;
const TEST_EXEC_ID: &str = "00025b46.63f8f39c.01.01";
const TEST_TIME: &str = "20230224  12:04:56";

// =============================================================================
// Response builders
// =============================================================================

// --- OrderStatus (msg 3) ---
//
// Server version >= MARKET_CAP_PRICE (131): version field is dropped, market_cap_price emitted.
// Tests run with SIZE_RULES (164), so we always emit the post-MARKET_CAP_PRICE shape.

#[derive(Clone, Debug)]
pub struct OrderStatusResponse {
    pub order_id: i32,
    pub status: String,
    pub filled: f64,
    pub remaining: f64,
    pub average_fill_price: Option<f64>,
    pub perm_id: i64,
    pub parent_id: i32,
    pub last_fill_price: Option<f64>,
    pub client_id: i32,
    pub why_held: String,
    pub market_cap_price: Option<f64>,
}

impl Default for OrderStatusResponse {
    fn default() -> Self {
        Self {
            order_id: 13,
            status: "Submitted".to_string(),
            filled: 0.0,
            remaining: 100.0,
            average_fill_price: Some(0.0),
            perm_id: TEST_PERM_ID,
            parent_id: 0,
            last_fill_price: Some(0.0),
            client_id: 100,
            why_held: String::new(),
            market_cap_price: Some(0.0),
        }
    }
}

impl OrderStatusResponse {
    pub fn order_id(mut self, v: i32) -> Self {
        self.order_id = v;
        self
    }
    pub fn status(mut self, v: impl Into<String>) -> Self {
        self.status = v.into();
        self
    }
    pub fn filled(mut self, v: f64) -> Self {
        self.filled = v;
        self
    }
    pub fn remaining(mut self, v: f64) -> Self {
        self.remaining = v;
        self
    }
    pub fn average_fill_price(mut self, v: Option<f64>) -> Self {
        self.average_fill_price = v;
        self
    }
    pub fn perm_id(mut self, v: i64) -> Self {
        self.perm_id = v;
        self
    }
    pub fn last_fill_price(mut self, v: Option<f64>) -> Self {
        self.last_fill_price = v;
        self
    }
    pub fn client_id(mut self, v: i32) -> Self {
        self.client_id = v;
        self
    }
    pub fn market_cap_price(mut self, v: Option<f64>) -> Self {
        self.market_cap_price = v;
        self
    }
}

fn opt_double_str(v: Option<f64>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}

impl ResponseEncoder for OrderStatusResponse {
    fn fields(&self) -> Vec<String> {
        vec![
            "3".to_string(),
            self.order_id.to_string(),
            self.status.clone(),
            self.filled.to_string(),
            self.remaining.to_string(),
            opt_double_str(self.average_fill_price),
            self.perm_id.to_string(),
            self.parent_id.to_string(),
            opt_double_str(self.last_fill_price),
            self.client_id.to_string(),
            self.why_held.clone(),
            opt_double_str(self.market_cap_price),
        ]
    }
}

impl ResponseProtoEncoder for OrderStatusResponse {
    type Proto = proto::OrderStatus;

    fn to_proto(&self) -> Self::Proto {
        proto::OrderStatus {
            order_id: Some(self.order_id),
            status: Some(self.status.clone()),
            filled: Some(self.filled.to_string()),
            remaining: Some(self.remaining.to_string()),
            avg_fill_price: self.average_fill_price,
            perm_id: Some(self.perm_id),
            parent_id: Some(self.parent_id),
            last_fill_price: self.last_fill_price,
            client_id: Some(self.client_id),
            why_held: if self.why_held.is_empty() { None } else { Some(self.why_held.clone()) },
            mkt_cap_price: self.market_cap_price,
        }
    }
}

// --- CommissionReport (msg 59) ---

#[derive(Clone, Debug)]
pub struct CommissionReportResponse {
    pub execution_id: String,
    pub commission: f64,
    pub currency: String,
    pub realized_pnl: Option<f64>,
    pub yields: Option<f64>,
    pub yield_redemption_date: String,
}

impl Default for CommissionReportResponse {
    fn default() -> Self {
        Self {
            execution_id: TEST_EXEC_ID.to_string(),
            commission: 1.0,
            currency: "USD".to_string(),
            realized_pnl: None,
            yields: None,
            yield_redemption_date: String::new(),
        }
    }
}

impl CommissionReportResponse {
    pub fn execution_id(mut self, v: impl Into<String>) -> Self {
        self.execution_id = v.into();
        self
    }
    pub fn commission(mut self, v: f64) -> Self {
        self.commission = v;
        self
    }
    pub fn currency(mut self, v: impl Into<String>) -> Self {
        self.currency = v.into();
        self
    }
    pub fn realized_pnl(mut self, v: Option<f64>) -> Self {
        self.realized_pnl = v;
        self
    }
    pub fn yields(mut self, v: Option<f64>) -> Self {
        self.yields = v;
        self
    }
}

impl ResponseEncoder for CommissionReportResponse {
    fn fields(&self) -> Vec<String> {
        vec![
            "59".to_string(),
            "1".to_string(),
            self.execution_id.clone(),
            self.commission.to_string(),
            self.currency.clone(),
            opt_double_str(self.realized_pnl),
            opt_double_str(self.yields),
            self.yield_redemption_date.clone(),
        ]
    }
}

impl ResponseProtoEncoder for CommissionReportResponse {
    type Proto = proto::CommissionAndFeesReport;

    fn to_proto(&self) -> Self::Proto {
        proto::CommissionAndFeesReport {
            exec_id: Some(self.execution_id.clone()),
            commission_and_fees: Some(self.commission),
            currency: Some(self.currency.clone()),
            realized_pnl: self.realized_pnl,
            bond_yield: self.yields,
            yield_redemption_date: if self.yield_redemption_date.is_empty() {
                None
            } else {
                Some(self.yield_redemption_date.clone())
            },
        }
    }
}

// --- ExecutionData (msg 11) ---
//
// Server version >= LAST_LIQUIDITY (136): version field dropped, model_code + last_liquidity emitted.
// PENDING_PRICE_REVISION (178) and SUBMITTER (198) are above SIZE_RULES (164), so those fields are NOT emitted.

#[derive(Clone, Debug)]
pub struct ExecutionDataResponse {
    pub request_id: i32,
    pub order_id: i32,
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
    pub execution_id: String,
    pub time: String,
    pub account: String,
    pub exec_exchange: String,
    pub side: String,
    pub shares: f64,
    pub price: f64,
    pub perm_id: i64,
    pub client_id: i32,
    pub liquidation: i32,
    pub cumulative_quantity: f64,
    pub average_price: f64,
    pub order_reference: String,
    pub ev_rule: String,
    pub ev_multiplier: Option<f64>,
    pub model_code: String,
    pub last_liquidity: i32,
}

impl Default for ExecutionDataResponse {
    fn default() -> Self {
        Self {
            request_id: -1,
            order_id: 13,
            contract_id: 76792991,
            symbol: "TSLA".to_string(),
            security_type: "STK".to_string(),
            last_trade_date_or_contract_month: String::new(),
            strike: 0.0,
            right: String::new(),
            multiplier: String::new(),
            exchange: "ISLAND".to_string(),
            currency: "USD".to_string(),
            local_symbol: "TSLA".to_string(),
            trading_class: "NMS".to_string(),
            execution_id: TEST_EXEC_ID.to_string(),
            time: TEST_TIME.to_string(),
            account: TEST_ACCOUNT.to_string(),
            exec_exchange: "ISLAND".to_string(),
            side: "BOT".to_string(),
            shares: 100.0,
            price: 196.52,
            perm_id: TEST_PERM_ID,
            client_id: 100,
            liquidation: 0,
            cumulative_quantity: 100.0,
            average_price: 196.52,
            order_reference: String::new(),
            ev_rule: String::new(),
            ev_multiplier: None,
            model_code: String::new(),
            last_liquidity: 2,
        }
    }
}

impl ExecutionDataResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn order_id(mut self, v: i32) -> Self {
        self.order_id = v;
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
    pub fn exchange(mut self, v: impl Into<String>) -> Self {
        self.exchange = v.into();
        self
    }
    pub fn execution_id(mut self, v: impl Into<String>) -> Self {
        self.execution_id = v.into();
        self
    }
    pub fn shares(mut self, v: f64) -> Self {
        self.shares = v;
        self
    }
    pub fn price(mut self, v: f64) -> Self {
        self.price = v;
        self
    }
    pub fn side(mut self, v: impl Into<String>) -> Self {
        self.side = v.into();
        self
    }
    pub fn perm_id(mut self, v: i64) -> Self {
        self.perm_id = v;
        self
    }
    pub fn last_liquidity(mut self, v: i32) -> Self {
        self.last_liquidity = v;
        self
    }
}

impl ResponseEncoder for ExecutionDataResponse {
    fn fields(&self) -> Vec<String> {
        vec![
            "11".to_string(),
            self.request_id.to_string(),
            self.order_id.to_string(),
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
            self.execution_id.clone(),
            self.time.clone(),
            self.account.clone(),
            self.exec_exchange.clone(),
            self.side.clone(),
            self.shares.to_string(),
            self.price.to_string(),
            self.perm_id.to_string(),
            self.client_id.to_string(),
            self.liquidation.to_string(),
            self.cumulative_quantity.to_string(),
            self.average_price.to_string(),
            self.order_reference.clone(),
            self.ev_rule.clone(),
            opt_double_str(self.ev_multiplier),
            self.model_code.clone(),
            self.last_liquidity.to_string(),
        ]
    }
}

impl ResponseProtoEncoder for ExecutionDataResponse {
    type Proto = proto::ExecutionDetails;

    fn to_proto(&self) -> Self::Proto {
        proto::ExecutionDetails {
            req_id: Some(self.request_id),
            contract: Some(proto::Contract {
                con_id: Some(self.contract_id),
                symbol: Some(self.symbol.clone()),
                sec_type: Some(self.security_type.clone()),
                exchange: Some(self.exchange.clone()),
                currency: Some(self.currency.clone()),
                local_symbol: Some(self.local_symbol.clone()),
                trading_class: Some(self.trading_class.clone()),
                ..Default::default()
            }),
            execution: Some(proto::Execution {
                order_id: Some(self.order_id),
                exec_id: Some(self.execution_id.clone()),
                time: Some(self.time.clone()),
                acct_number: Some(self.account.clone()),
                exchange: Some(self.exec_exchange.clone()),
                side: Some(self.side.clone()),
                shares: Some(self.shares.to_string()),
                price: Some(self.price),
                perm_id: Some(self.perm_id),
                client_id: Some(self.client_id),
                is_liquidation: Some(self.liquidation != 0),
                cum_qty: Some(self.cumulative_quantity.to_string()),
                avg_price: Some(self.average_price),
                order_ref: some_str(&self.order_reference),
                ev_rule: some_str(&self.ev_rule),
                ev_multiplier: self.ev_multiplier,
                model_code: some_str(&self.model_code),
                last_liquidity: Some(self.last_liquidity),
                ..Default::default()
            }),
        }
    }
}

// --- OpenOrderEnd (msg 53) ---

#[derive(Clone, Copy, Debug, Default)]
pub struct OpenOrderEndResponse;

impl ResponseEncoder for OpenOrderEndResponse {
    fn fields(&self) -> Vec<String> {
        vec!["53".to_string(), "1".to_string()]
    }
}

impl ResponseProtoEncoder for OpenOrderEndResponse {
    type Proto = proto::OpenOrdersEnd;

    fn to_proto(&self) -> Self::Proto {
        proto::OpenOrdersEnd {}
    }
}

// --- ExecutionDataEnd (msg 55) ---

#[derive(Clone, Copy, Debug)]
pub struct ExecutionDataEndResponse {
    pub request_id: i32,
}

impl Default for ExecutionDataEndResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
        }
    }
}

impl ExecutionDataEndResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
}

impl ResponseEncoder for ExecutionDataEndResponse {
    fn fields(&self) -> Vec<String> {
        vec!["55".to_string(), "1".to_string(), self.request_id.to_string()]
    }
}

impl ResponseProtoEncoder for ExecutionDataEndResponse {
    type Proto = proto::ExecutionDetailsEnd;

    fn to_proto(&self) -> Self::Proto {
        proto::ExecutionDetailsEnd {
            req_id: Some(self.request_id),
        }
    }
}

// --- CompletedOrdersEnd (msg 102) ---

#[derive(Clone, Copy, Debug, Default)]
pub struct CompletedOrdersEndResponse;

impl ResponseEncoder for CompletedOrdersEndResponse {
    fn fields(&self) -> Vec<String> {
        vec!["102".to_string()]
    }
}

impl ResponseProtoEncoder for CompletedOrdersEndResponse {
    type Proto = proto::CompletedOrdersEnd;

    fn to_proto(&self) -> Self::Proto {
        proto::CompletedOrdersEnd {}
    }
}

// =============================================================================
// Request builders
// =============================================================================

#[derive(Clone, Debug)]
pub struct PlaceOrderRequestBuilder {
    pub order_id: i32,
    pub contract: Contract,
    pub order: Order,
}

impl Default for PlaceOrderRequestBuilder {
    fn default() -> Self {
        Self {
            order_id: 13,
            contract: Contract::default(),
            order: Order::default(),
        }
    }
}

impl PlaceOrderRequestBuilder {
    pub fn order_id(mut self, v: i32) -> Self {
        self.order_id = v;
        self
    }
    pub fn contract(mut self, contract: &Contract) -> Self {
        self.contract = contract.clone();
        self
    }
    pub fn order(mut self, order: &Order) -> Self {
        self.order = order.clone();
        self
    }
}

impl RequestEncoder for PlaceOrderRequestBuilder {
    type Proto = proto::PlaceOrderRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::PlaceOrder;

    fn to_proto(&self) -> Self::Proto {
        proto::PlaceOrderRequest {
            order_id: Some(self.order_id),
            contract: Some(encode_contract_with_order(&self.contract, Some(&self.order))),
            order: Some(encode_order(&self.order)),
            attached_orders: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CancelOrderRequestBuilder {
    pub order_id: i32,
    pub manual_order_cancel_time: String,
}

impl Default for CancelOrderRequestBuilder {
    fn default() -> Self {
        Self {
            order_id: 13,
            manual_order_cancel_time: String::new(),
        }
    }
}

impl CancelOrderRequestBuilder {
    pub fn order_id(mut self, v: i32) -> Self {
        self.order_id = v;
        self
    }
    pub fn manual_order_cancel_time(mut self, v: impl Into<String>) -> Self {
        self.manual_order_cancel_time = v.into();
        self
    }
}

impl RequestEncoder for CancelOrderRequestBuilder {
    type Proto = proto::CancelOrderRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::CancelOrder;

    fn to_proto(&self) -> Self::Proto {
        proto::CancelOrderRequest {
            order_id: Some(self.order_id),
            order_cancel: Some(encode_order_cancel(&self.manual_order_cancel_time)),
        }
    }
}

empty_request_builder!(OpenOrdersRequestBuilder, OpenOrdersRequest, OutgoingMessages::RequestOpenOrders);
empty_request_builder!(AllOpenOrdersRequestBuilder, AllOpenOrdersRequest, OutgoingMessages::RequestAllOpenOrders);

#[derive(Clone, Copy, Debug)]
pub struct AutoOpenOrdersRequestBuilder {
    pub auto_bind: bool,
}

impl Default for AutoOpenOrdersRequestBuilder {
    fn default() -> Self {
        Self { auto_bind: true }
    }
}

impl AutoOpenOrdersRequestBuilder {
    pub fn auto_bind(mut self, v: bool) -> Self {
        self.auto_bind = v;
        self
    }
}

impl RequestEncoder for AutoOpenOrdersRequestBuilder {
    type Proto = proto::AutoOpenOrdersRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestAutoOpenOrders;

    fn to_proto(&self) -> Self::Proto {
        proto::AutoOpenOrdersRequest {
            auto_bind: some_bool(self.auto_bind),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CompletedOrdersRequestBuilder {
    pub api_only: bool,
}

impl Default for CompletedOrdersRequestBuilder {
    fn default() -> Self {
        Self { api_only: true }
    }
}

impl CompletedOrdersRequestBuilder {
    pub fn api_only(mut self, v: bool) -> Self {
        self.api_only = v;
        self
    }
}

impl RequestEncoder for CompletedOrdersRequestBuilder {
    type Proto = proto::CompletedOrdersRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestCompletedOrders;

    fn to_proto(&self) -> Self::Proto {
        proto::CompletedOrdersRequest {
            api_only: some_bool(self.api_only),
        }
    }
}

#[derive(Debug)]
pub struct ExecutionsRequestBuilder {
    pub request_id: i32,
    pub filter: ExecutionFilter,
}

impl Default for ExecutionsRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            filter: ExecutionFilter::default(),
        }
    }
}

impl ExecutionsRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn filter(mut self, filter: ExecutionFilter) -> Self {
        self.filter = filter;
        self
    }
}

impl RequestEncoder for ExecutionsRequestBuilder {
    type Proto = proto::ExecutionRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestExecutions;

    fn to_proto(&self) -> Self::Proto {
        proto::ExecutionRequest {
            req_id: Some(self.request_id),
            execution_filter: Some(encode_execution_filter(&self.filter)),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct GlobalCancelRequestBuilder {
    pub manual_order_cancel_time: String,
}

impl GlobalCancelRequestBuilder {
    pub fn manual_order_cancel_time(mut self, v: impl Into<String>) -> Self {
        self.manual_order_cancel_time = v.into();
        self
    }
}

impl RequestEncoder for GlobalCancelRequestBuilder {
    type Proto = proto::GlobalCancelRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestGlobalCancel;

    fn to_proto(&self) -> Self::Proto {
        proto::GlobalCancelRequest {
            order_cancel: Some(encode_order_cancel(&self.manual_order_cancel_time)),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NextValidOrderIdRequestBuilder;

impl RequestEncoder for NextValidOrderIdRequestBuilder {
    type Proto = proto::IdsRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestIds;

    fn to_proto(&self) -> Self::Proto {
        proto::IdsRequest { num_ids: Some(0) }
    }
}

#[derive(Clone, Debug)]
pub struct ExerciseOptionsRequestBuilder {
    pub order_id: i32,
    pub contract: Contract,
    pub exercise_action: ExerciseAction,
    pub exercise_quantity: i32,
    pub account: String,
    pub r#override: bool,
    pub manual_order_time: Option<String>,
}

impl Default for ExerciseOptionsRequestBuilder {
    fn default() -> Self {
        Self {
            order_id: 13,
            contract: Contract::default(),
            exercise_action: ExerciseAction::Exercise,
            exercise_quantity: 1,
            account: String::new(),
            r#override: false,
            manual_order_time: None,
        }
    }
}

impl ExerciseOptionsRequestBuilder {
    pub fn order_id(mut self, v: i32) -> Self {
        self.order_id = v;
        self
    }
    pub fn contract(mut self, contract: &Contract) -> Self {
        self.contract = contract.clone();
        self
    }
    pub fn exercise_action(mut self, v: ExerciseAction) -> Self {
        self.exercise_action = v;
        self
    }
    pub fn exercise_quantity(mut self, v: i32) -> Self {
        self.exercise_quantity = v;
        self
    }
    pub fn account(mut self, v: impl Into<String>) -> Self {
        self.account = v.into();
        self
    }
    pub fn override_(mut self, v: bool) -> Self {
        self.r#override = v;
        self
    }
}

impl RequestEncoder for ExerciseOptionsRequestBuilder {
    type Proto = proto::ExerciseOptionsRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::ExerciseOptions;

    fn to_proto(&self) -> Self::Proto {
        proto::ExerciseOptionsRequest {
            order_id: Some(self.order_id),
            contract: Some(encode_contract(&self.contract)),
            exercise_action: Some(self.exercise_action as i32),
            exercise_quantity: Some(self.exercise_quantity),
            account: some_str(&self.account),
            r#override: some_bool(self.r#override),
            manual_order_time: self.manual_order_time.clone(),
            customer_account: None,
            professional_customer: None,
        }
    }
}

// =============================================================================
// Entry-point functions
// =============================================================================

pub fn order_status() -> OrderStatusResponse {
    OrderStatusResponse::default()
}

pub fn commission_report() -> CommissionReportResponse {
    CommissionReportResponse::default()
}

pub fn execution_data() -> ExecutionDataResponse {
    ExecutionDataResponse::default()
}

pub fn open_order_end() -> OpenOrderEndResponse {
    OpenOrderEndResponse
}

pub fn execution_data_end() -> ExecutionDataEndResponse {
    ExecutionDataEndResponse::default()
}

pub fn completed_orders_end() -> CompletedOrdersEndResponse {
    CompletedOrdersEndResponse
}

pub fn place_order_request() -> PlaceOrderRequestBuilder {
    PlaceOrderRequestBuilder::default()
}

pub fn cancel_order_request() -> CancelOrderRequestBuilder {
    CancelOrderRequestBuilder::default()
}

pub fn open_orders_request() -> OpenOrdersRequestBuilder {
    OpenOrdersRequestBuilder
}

pub fn all_open_orders_request() -> AllOpenOrdersRequestBuilder {
    AllOpenOrdersRequestBuilder
}

pub fn auto_open_orders_request() -> AutoOpenOrdersRequestBuilder {
    AutoOpenOrdersRequestBuilder::default()
}

pub fn completed_orders_request() -> CompletedOrdersRequestBuilder {
    CompletedOrdersRequestBuilder::default()
}

pub fn executions_request() -> ExecutionsRequestBuilder {
    ExecutionsRequestBuilder::default()
}

pub fn global_cancel_request() -> GlobalCancelRequestBuilder {
    GlobalCancelRequestBuilder::default()
}

pub fn next_valid_order_id_request() -> NextValidOrderIdRequestBuilder {
    NextValidOrderIdRequestBuilder
}

pub fn exercise_options_request() -> ExerciseOptionsRequestBuilder {
    ExerciseOptionsRequestBuilder::default()
}

#[cfg(test)]
#[path = "orders_tests.rs"]
mod tests;
