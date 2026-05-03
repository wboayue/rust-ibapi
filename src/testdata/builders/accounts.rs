//! Builders for accounts-domain response and request messages.
//!
//! Excludes positions, which live in [`super::positions`].

use super::{RequestEncoder, ResponseEncoder, ResponseProtoEncoder};
use crate::common::test_utils::helpers::constants::{TEST_ACCOUNT, TEST_CONTRACT_ID, TEST_MODEL_CODE, TEST_TICKER_ID};
use crate::messages::OutgoingMessages;
use crate::proto;

// =============================================================================
// Response builders
// =============================================================================

// --- ManagedAccounts (msg 15) ---

#[derive(Clone, Debug)]
pub struct ManagedAccountsResponse {
    pub accounts: Vec<String>,
}

impl Default for ManagedAccountsResponse {
    fn default() -> Self {
        Self {
            accounts: vec![TEST_ACCOUNT.to_string(), "DU7654321".to_string()],
        }
    }
}

impl ManagedAccountsResponse {
    pub fn accounts<I, S>(mut self, accounts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.accounts = accounts.into_iter().map(Into::into).collect();
        self
    }
}

impl ResponseEncoder for ManagedAccountsResponse {
    fn fields(&self) -> Vec<String> {
        vec!["15".to_string(), "1".to_string(), self.accounts.join(",")]
    }
}

impl ResponseProtoEncoder for ManagedAccountsResponse {
    type Proto = proto::ManagedAccounts;

    fn to_proto(&self) -> Self::Proto {
        proto::ManagedAccounts {
            accounts_list: Some(self.accounts.join(",")),
        }
    }
}

// --- AccountSummary (msg 63) ---

#[derive(Clone, Debug)]
pub struct AccountSummaryResponse {
    pub request_id: i32,
    pub account: String,
    pub tag: String,
    pub value: String,
    pub currency: String,
}

impl Default for AccountSummaryResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_TICKER_ID,
            account: TEST_ACCOUNT.to_string(),
            tag: "AccountType".to_string(),
            value: "FA".to_string(),
            currency: String::new(),
        }
    }
}

impl AccountSummaryResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn account(mut self, v: impl Into<String>) -> Self {
        self.account = v.into();
        self
    }
    pub fn tag(mut self, v: impl Into<String>) -> Self {
        self.tag = v.into();
        self
    }
    pub fn value(mut self, v: impl Into<String>) -> Self {
        self.value = v.into();
        self
    }
    pub fn currency(mut self, v: impl Into<String>) -> Self {
        self.currency = v.into();
        self
    }
}

impl ResponseProtoEncoder for AccountSummaryResponse {
    type Proto = proto::AccountSummary;

    fn to_proto(&self) -> Self::Proto {
        proto::AccountSummary {
            req_id: Some(self.request_id),
            account: Some(self.account.clone()),
            tag: Some(self.tag.clone()),
            value: Some(self.value.clone()),
            currency: Some(self.currency.clone()),
        }
    }
}

impl ResponseEncoder for AccountSummaryResponse {
    fn fields(&self) -> Vec<String> {
        vec![
            "63".to_string(),
            "1".to_string(),
            self.request_id.to_string(),
            self.account.clone(),
            self.tag.clone(),
            self.value.clone(),
            self.currency.clone(),
        ]
    }
}

// --- AccountSummaryEnd (msg 64) ---

request_id_response_builder!(AccountSummaryEndResponse, "64", AccountSummaryEnd);

// --- AccountValue (msg 6) ---

#[derive(Clone, Debug)]
pub struct AccountValueResponse {
    pub key: String,
    pub value: String,
    pub currency: String,
    pub account: Option<String>,
}

impl Default for AccountValueResponse {
    fn default() -> Self {
        Self {
            key: "CashBalance".to_string(),
            value: "1000.00".to_string(),
            currency: "USD".to_string(),
            account: None,
        }
    }
}

impl AccountValueResponse {
    pub fn key(mut self, v: impl Into<String>) -> Self {
        self.key = v.into();
        self
    }
    pub fn value(mut self, v: impl Into<String>) -> Self {
        self.value = v.into();
        self
    }
    pub fn currency(mut self, v: impl Into<String>) -> Self {
        self.currency = v.into();
        self
    }
    pub fn account(mut self, v: impl Into<String>) -> Self {
        self.account = Some(v.into());
        self
    }
}

impl ResponseProtoEncoder for AccountValueResponse {
    type Proto = proto::AccountValue;

    fn to_proto(&self) -> Self::Proto {
        proto::AccountValue {
            key: Some(self.key.clone()),
            value: Some(self.value.clone()),
            currency: Some(self.currency.clone()),
            account_name: self.account.clone(),
        }
    }
}

impl ResponseEncoder for AccountValueResponse {
    fn fields(&self) -> Vec<String> {
        let version = if self.account.is_some() { "2" } else { "1" };
        let mut fields = vec![
            "6".to_string(),
            version.to_string(),
            self.key.clone(),
            self.value.clone(),
            self.currency.clone(),
        ];
        if let Some(account) = &self.account {
            fields.push(account.clone());
        }
        fields
    }
}

// --- AccountDownloadEnd (msg 54) ---

#[derive(Clone, Debug)]
pub struct AccountDownloadEndResponse {
    pub account: String,
}

impl Default for AccountDownloadEndResponse {
    fn default() -> Self {
        Self {
            account: TEST_ACCOUNT.to_string(),
        }
    }
}

impl AccountDownloadEndResponse {
    pub fn account(mut self, v: impl Into<String>) -> Self {
        self.account = v.into();
        self
    }
}

impl ResponseProtoEncoder for AccountDownloadEndResponse {
    type Proto = proto::AccountDataEnd;

    fn to_proto(&self) -> Self::Proto {
        proto::AccountDataEnd {
            account_name: Some(self.account.clone()),
        }
    }
}

impl ResponseEncoder for AccountDownloadEndResponse {
    fn fields(&self) -> Vec<String> {
        vec!["54".to_string(), "1".to_string(), self.account.clone()]
    }
}

// --- AccountUpdateMulti (msg 73) ---

#[derive(Clone, Debug)]
pub struct AccountUpdateMultiResponse {
    pub request_id: i32,
    pub account: String,
    pub model_code: String,
    pub key: String,
    pub value: String,
    pub currency: String,
}

impl Default for AccountUpdateMultiResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_TICKER_ID,
            account: TEST_ACCOUNT.to_string(),
            model_code: String::new(),
            key: "CashBalance".to_string(),
            value: "94629.71".to_string(),
            currency: "USD".to_string(),
        }
    }
}

impl AccountUpdateMultiResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn account(mut self, v: impl Into<String>) -> Self {
        self.account = v.into();
        self
    }
    pub fn model_code(mut self, v: impl Into<String>) -> Self {
        self.model_code = v.into();
        self
    }
    pub fn key(mut self, v: impl Into<String>) -> Self {
        self.key = v.into();
        self
    }
    pub fn value(mut self, v: impl Into<String>) -> Self {
        self.value = v.into();
        self
    }
    pub fn currency(mut self, v: impl Into<String>) -> Self {
        self.currency = v.into();
        self
    }
}

impl ResponseProtoEncoder for AccountUpdateMultiResponse {
    type Proto = proto::AccountUpdateMulti;

    fn to_proto(&self) -> Self::Proto {
        proto::AccountUpdateMulti {
            req_id: Some(self.request_id),
            account: Some(self.account.clone()),
            model_code: Some(self.model_code.clone()),
            key: Some(self.key.clone()),
            value: Some(self.value.clone()),
            currency: Some(self.currency.clone()),
        }
    }
}

impl ResponseEncoder for AccountUpdateMultiResponse {
    fn fields(&self) -> Vec<String> {
        vec![
            "73".to_string(),
            "1".to_string(),
            self.request_id.to_string(),
            self.account.clone(),
            self.model_code.clone(),
            self.key.clone(),
            self.value.clone(),
            self.currency.clone(),
        ]
    }
}

// --- AccountUpdateMultiEnd (msg 74) ---

request_id_response_builder!(AccountUpdateMultiEndResponse, "74", AccountUpdateMultiEnd);

// --- FamilyCodes (msg 78) ---

#[derive(Clone, Debug)]
pub struct FamilyCodeEntry {
    pub account_id: String,
    pub family_code: String,
}

#[derive(Clone, Debug, Default)]
pub struct FamilyCodesResponse {
    pub codes: Vec<FamilyCodeEntry>,
}

impl FamilyCodesResponse {
    pub fn codes<I>(mut self, codes: I) -> Self
    where
        I: IntoIterator<Item = FamilyCodeEntry>,
    {
        self.codes = codes.into_iter().collect();
        self
    }

    pub fn push(mut self, account_id: impl Into<String>, family_code: impl Into<String>) -> Self {
        self.codes.push(FamilyCodeEntry {
            account_id: account_id.into(),
            family_code: family_code.into(),
        });
        self
    }
}

impl ResponseProtoEncoder for FamilyCodesResponse {
    type Proto = proto::FamilyCodes;

    fn to_proto(&self) -> Self::Proto {
        proto::FamilyCodes {
            family_codes: self
                .codes
                .iter()
                .map(|c| proto::FamilyCode {
                    account_id: Some(c.account_id.clone()),
                    family_code: Some(c.family_code.clone()),
                })
                .collect(),
        }
    }
}

impl ResponseEncoder for FamilyCodesResponse {
    fn fields(&self) -> Vec<String> {
        let mut fields = vec!["78".to_string(), self.codes.len().to_string()];
        for code in &self.codes {
            fields.push(code.account_id.clone());
            fields.push(code.family_code.clone());
        }
        fields
    }
}

// --- CurrentTime (msg 49) ---

#[derive(Clone, Debug)]
pub struct CurrentTimeResponse {
    pub timestamp: i64,
}

impl Default for CurrentTimeResponse {
    fn default() -> Self {
        // 2023-03-15 14:20:00 UTC
        Self { timestamp: 1678890000 }
    }
}

impl CurrentTimeResponse {
    pub fn timestamp(mut self, v: i64) -> Self {
        self.timestamp = v;
        self
    }
}

impl ResponseProtoEncoder for CurrentTimeResponse {
    type Proto = proto::CurrentTime;

    fn to_proto(&self) -> Self::Proto {
        proto::CurrentTime {
            current_time: Some(self.timestamp),
        }
    }
}

impl ResponseEncoder for CurrentTimeResponse {
    fn fields(&self) -> Vec<String> {
        vec!["49".to_string(), "1".to_string(), self.timestamp.to_string()]
    }
}

// --- PnL (msg 94) ---

#[derive(Clone, Debug)]
pub struct PnLResponse {
    pub request_id: i32,
    pub daily_pnl: f64,
    pub unrealized_pnl: Option<f64>,
    pub realized_pnl: Option<f64>,
}

impl Default for PnLResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_TICKER_ID,
            daily_pnl: 1234.56,
            unrealized_pnl: Some(500.0),
            realized_pnl: Some(250.0),
        }
    }
}

impl PnLResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn daily_pnl(mut self, v: f64) -> Self {
        self.daily_pnl = v;
        self
    }
    pub fn unrealized_pnl(mut self, v: Option<f64>) -> Self {
        self.unrealized_pnl = v;
        self
    }
    pub fn realized_pnl(mut self, v: Option<f64>) -> Self {
        self.realized_pnl = v;
        self
    }
}

impl ResponseProtoEncoder for PnLResponse {
    type Proto = proto::PnL;

    fn to_proto(&self) -> Self::Proto {
        proto::PnL {
            req_id: Some(self.request_id),
            daily_pn_l: Some(self.daily_pnl),
            unrealized_pn_l: self.unrealized_pnl,
            realized_pn_l: self.realized_pnl,
        }
    }
}

impl ResponseEncoder for PnLResponse {
    fn fields(&self) -> Vec<String> {
        let mut fields = vec!["94".to_string(), self.request_id.to_string(), self.daily_pnl.to_string()];
        if let Some(u) = self.unrealized_pnl {
            fields.push(u.to_string());
        }
        if let Some(r) = self.realized_pnl {
            fields.push(r.to_string());
        }
        fields
    }
}

// --- PnLSingle (msg 95) ---

#[derive(Clone, Debug)]
pub struct PnLSingleResponse {
    pub request_id: i32,
    pub position: f64,
    pub daily_pnl: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub value: f64,
}

impl Default for PnLSingleResponse {
    fn default() -> Self {
        Self {
            request_id: TEST_TICKER_ID,
            position: 100.0,
            daily_pnl: 50.0,
            unrealized_pnl: 25.0,
            realized_pnl: 10.0,
            value: 12345.67,
        }
    }
}

impl PnLSingleResponse {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn position(mut self, v: f64) -> Self {
        self.position = v;
        self
    }
    pub fn daily_pnl(mut self, v: f64) -> Self {
        self.daily_pnl = v;
        self
    }
    pub fn unrealized_pnl(mut self, v: f64) -> Self {
        self.unrealized_pnl = v;
        self
    }
    pub fn realized_pnl(mut self, v: f64) -> Self {
        self.realized_pnl = v;
        self
    }
    pub fn value(mut self, v: f64) -> Self {
        self.value = v;
        self
    }
}

impl ResponseProtoEncoder for PnLSingleResponse {
    type Proto = proto::PnLSingle;

    fn to_proto(&self) -> Self::Proto {
        proto::PnLSingle {
            req_id: Some(self.request_id),
            position: Some(self.position.to_string()),
            daily_pn_l: Some(self.daily_pnl),
            unrealized_pn_l: Some(self.unrealized_pnl),
            realized_pn_l: Some(self.realized_pnl),
            value: Some(self.value),
        }
    }
}

impl ResponseEncoder for PnLSingleResponse {
    fn fields(&self) -> Vec<String> {
        vec![
            "95".to_string(),
            self.request_id.to_string(),
            self.position.to_string(),
            self.daily_pnl.to_string(),
            self.unrealized_pnl.to_string(),
            self.realized_pnl.to_string(),
            self.value.to_string(),
        ]
    }
}

// =============================================================================
// Request builders
// =============================================================================

empty_request_builder!(
    ManagedAccountsRequestBuilder,
    ManagedAccountsRequest,
    OutgoingMessages::RequestManagedAccounts
);

#[derive(Clone, Debug)]
pub struct AccountSummaryRequestBuilder {
    pub request_id: i32,
    pub group: String,
    pub tags: Vec<String>,
}

impl Default for AccountSummaryRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_TICKER_ID,
            group: "All".to_string(),
            tags: vec!["AccountType".to_string()],
        }
    }
}

impl AccountSummaryRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn group(mut self, v: impl Into<String>) -> Self {
        self.group = v.into();
        self
    }
    pub fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }
}

impl RequestEncoder for AccountSummaryRequestBuilder {
    type Proto = proto::AccountSummaryRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestAccountSummary;

    fn to_proto(&self) -> Self::Proto {
        proto::AccountSummaryRequest {
            req_id: Some(self.request_id),
            group: Some(self.group.clone()),
            tags: Some(self.tags.join(",")),
        }
    }
}

single_req_id_request_builder!(CancelAccountSummaryBuilder, CancelAccountSummary, OutgoingMessages::CancelAccountSummary);

#[derive(Clone, Debug)]
pub struct AccountUpdatesRequestBuilder {
    pub subscribe: bool,
    pub account: Option<String>,
}

impl Default for AccountUpdatesRequestBuilder {
    fn default() -> Self {
        Self {
            subscribe: true,
            account: Some(TEST_ACCOUNT.to_string()),
        }
    }
}

impl AccountUpdatesRequestBuilder {
    pub fn subscribe(mut self, v: bool) -> Self {
        self.subscribe = v;
        self
    }
    pub fn account(mut self, v: impl Into<String>) -> Self {
        self.account = Some(v.into());
        self
    }
    pub fn no_account(mut self) -> Self {
        self.account = None;
        self
    }
}

impl RequestEncoder for AccountUpdatesRequestBuilder {
    type Proto = proto::AccountDataRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestAccountData;

    fn to_proto(&self) -> Self::Proto {
        proto::AccountDataRequest {
            subscribe: Some(self.subscribe),
            acct_code: self.account.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AccountUpdatesMultiRequestBuilder {
    pub request_id: i32,
    pub account: Option<String>,
    pub model_code: Option<String>,
    pub ledger_and_nlv: bool,
}

impl Default for AccountUpdatesMultiRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_TICKER_ID,
            account: Some(TEST_ACCOUNT.to_string()),
            model_code: None,
            ledger_and_nlv: true,
        }
    }
}

impl AccountUpdatesMultiRequestBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }
    pub fn account(mut self, v: impl Into<String>) -> Self {
        self.account = Some(v.into());
        self
    }
    pub fn no_account(mut self) -> Self {
        self.account = None;
        self
    }
    pub fn model_code(mut self, v: impl Into<String>) -> Self {
        self.model_code = Some(v.into());
        self
    }
    pub fn ledger_and_nlv(mut self, v: bool) -> Self {
        self.ledger_and_nlv = v;
        self
    }
}

impl RequestEncoder for AccountUpdatesMultiRequestBuilder {
    type Proto = proto::AccountUpdatesMultiRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestAccountUpdatesMulti;

    fn to_proto(&self) -> Self::Proto {
        proto::AccountUpdatesMultiRequest {
            req_id: Some(self.request_id),
            account: self.account.clone(),
            model_code: self.model_code.clone(),
            ledger_and_nlv: Some(self.ledger_and_nlv),
        }
    }
}

single_req_id_request_builder!(
    CancelAccountUpdatesMultiBuilder,
    CancelAccountUpdatesMulti,
    OutgoingMessages::CancelAccountUpdatesMulti
);

#[derive(Clone, Debug)]
pub struct PnLRequestBuilder {
    pub request_id: i32,
    pub account: String,
    pub model_code: Option<String>,
}

impl Default for PnLRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_TICKER_ID,
            account: TEST_ACCOUNT.to_string(),
            model_code: Some(TEST_MODEL_CODE.to_string()),
        }
    }
}

impl PnLRequestBuilder {
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
    pub fn no_model_code(mut self) -> Self {
        self.model_code = None;
        self
    }
}

impl RequestEncoder for PnLRequestBuilder {
    type Proto = proto::PnLRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestPnL;

    fn to_proto(&self) -> Self::Proto {
        proto::PnLRequest {
            req_id: Some(self.request_id),
            account: Some(self.account.clone()),
            model_code: self.model_code.clone(),
        }
    }
}

single_req_id_request_builder!(CancelPnLBuilder, CancelPnL, OutgoingMessages::CancelPnL);

#[derive(Clone, Debug)]
pub struct PnLSingleRequestBuilder {
    pub request_id: i32,
    pub account: String,
    pub contract_id: i32,
    pub model_code: Option<String>,
}

impl Default for PnLSingleRequestBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_TICKER_ID,
            account: TEST_ACCOUNT.to_string(),
            contract_id: TEST_CONTRACT_ID,
            model_code: Some(TEST_MODEL_CODE.to_string()),
        }
    }
}

impl PnLSingleRequestBuilder {
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
    pub fn model_code(mut self, v: impl Into<String>) -> Self {
        self.model_code = Some(v.into());
        self
    }
    pub fn no_model_code(mut self) -> Self {
        self.model_code = None;
        self
    }
}

impl RequestEncoder for PnLSingleRequestBuilder {
    type Proto = proto::PnLSingleRequest;
    const MSG_ID: OutgoingMessages = OutgoingMessages::RequestPnLSingle;

    fn to_proto(&self) -> Self::Proto {
        proto::PnLSingleRequest {
            req_id: Some(self.request_id),
            account: Some(self.account.clone()),
            model_code: self.model_code.clone(),
            con_id: Some(self.contract_id),
        }
    }
}

single_req_id_request_builder!(CancelPnLSingleBuilder, CancelPnLSingle, OutgoingMessages::CancelPnLSingle);

empty_request_builder!(FamilyCodesRequestBuilder, FamilyCodesRequest, OutgoingMessages::RequestFamilyCodes);
empty_request_builder!(CurrentTimeRequestBuilder, CurrentTimeRequest, OutgoingMessages::RequestCurrentTime);

// =============================================================================
// Entry-point functions
// =============================================================================

pub fn managed_accounts() -> ManagedAccountsResponse {
    ManagedAccountsResponse::default()
}

pub fn account_summary() -> AccountSummaryResponse {
    AccountSummaryResponse::default()
}

pub fn account_summary_end() -> AccountSummaryEndResponse {
    AccountSummaryEndResponse::default()
}

pub fn account_value() -> AccountValueResponse {
    AccountValueResponse::default()
}

pub fn account_download_end() -> AccountDownloadEndResponse {
    AccountDownloadEndResponse::default()
}

pub fn account_update_multi() -> AccountUpdateMultiResponse {
    AccountUpdateMultiResponse::default()
}

pub fn account_update_multi_end() -> AccountUpdateMultiEndResponse {
    AccountUpdateMultiEndResponse::default()
}

pub fn family_codes() -> FamilyCodesResponse {
    FamilyCodesResponse::default()
}

pub fn current_time() -> CurrentTimeResponse {
    CurrentTimeResponse::default()
}

pub fn pnl() -> PnLResponse {
    PnLResponse::default()
}

pub fn pnl_single() -> PnLSingleResponse {
    PnLSingleResponse::default()
}

pub fn request_managed_accounts() -> ManagedAccountsRequestBuilder {
    ManagedAccountsRequestBuilder
}

pub fn request_account_summary() -> AccountSummaryRequestBuilder {
    AccountSummaryRequestBuilder::default()
}

pub fn cancel_account_summary() -> CancelAccountSummaryBuilder {
    CancelAccountSummaryBuilder::default()
}

pub fn request_account_updates() -> AccountUpdatesRequestBuilder {
    AccountUpdatesRequestBuilder::default()
}

pub fn cancel_account_updates() -> AccountUpdatesRequestBuilder {
    AccountUpdatesRequestBuilder {
        subscribe: false,
        account: None,
    }
}

pub fn request_account_updates_multi() -> AccountUpdatesMultiRequestBuilder {
    AccountUpdatesMultiRequestBuilder::default()
}

pub fn cancel_account_updates_multi() -> CancelAccountUpdatesMultiBuilder {
    CancelAccountUpdatesMultiBuilder::default()
}

pub fn request_pnl() -> PnLRequestBuilder {
    PnLRequestBuilder::default()
}

pub fn cancel_pnl() -> CancelPnLBuilder {
    CancelPnLBuilder::default()
}

pub fn request_pnl_single() -> PnLSingleRequestBuilder {
    PnLSingleRequestBuilder::default()
}

pub fn cancel_pnl_single() -> CancelPnLSingleBuilder {
    CancelPnLSingleBuilder::default()
}

pub fn request_family_codes() -> FamilyCodesRequestBuilder {
    FamilyCodesRequestBuilder
}

pub fn request_current_time() -> CurrentTimeRequestBuilder {
    CurrentTimeRequestBuilder
}

#[cfg(test)]
#[path = "accounts_tests.rs"]
mod tests;
