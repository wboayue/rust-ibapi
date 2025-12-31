//! Table-driven test data for accounts module tests

use crate::accounts::types::*;
use crate::server_versions;
use time::macros::datetime;
use time::OffsetDateTime;

/// Test case for server version compatibility checking
#[derive(Debug, Clone)]
pub struct VersionTestCase {
    pub function_name: &'static str,
    pub required_version: i32,
}

/// Test case for managed accounts scenarios
#[derive(Debug, Clone)]
pub struct ManagedAccountsTestCase {
    pub scenario: &'static str,
    pub responses: Vec<String>,
    pub expected: Vec<String>,
    pub description: &'static str,
}

/// Test case for server time scenarios
#[derive(Debug, Clone)]
pub struct ServerTimeTestCase {
    pub scenario: &'static str,
    pub responses: Vec<String>,
    pub expected_result: Result<OffsetDateTime, &'static str>,
    pub expected_request: &'static str,
}

/// Test case for contract ID edge cases
#[derive(Debug, Clone)]
pub struct ContractIdTestCase {
    pub description: &'static str,
    pub contract_id: ContractId,
    pub expected_pattern: String,
}

/// Test case for PnL parameter combinations
#[derive(Debug, Clone)]
pub struct PnLTestCase {
    pub description: &'static str,
    pub model_code: Option<String>,
    pub expected_pattern: &'static str,
}

/// Test case for positions multi parameter combinations
#[derive(Debug, Clone)]
pub struct PositionsMultiTestCase {
    pub description: &'static str,
    pub account: Option<String>,
    pub model_code: Option<String>,
}

/// Test case for account summary tag combinations
#[derive(Debug, Clone)]
pub struct AccountSummaryTagTestCase {
    pub description: &'static str,
    pub group: String,
    pub tags: Vec<&'static str>,
    pub expected_tag_encoding: Option<&'static str>,
    pub should_succeed: bool,
    pub expect_responses: bool,
}

/// Test case for subscription lifecycle testing
#[derive(Debug, Clone)]
pub struct SubscriptionLifecycleTestCase {
    pub description: &'static str,
    pub subscription_type: SubscriptionType,
    pub expected_subscribe_pattern: &'static str,
    pub expected_cancel_pattern: &'static str,
}

/// Types of subscriptions for lifecycle testing
#[derive(Debug, Clone)]
pub enum SubscriptionType {
    PnL {
        account: String,
        model_code: Option<String>,
    },
    PnLSingle {
        account: String,
        contract_id: i32,
        model_code: Option<String>,
    },
    Positions,
    PositionsMulti {
        account: Option<String>,
        model_code: Option<String>,
    },
    AccountSummary {
        group: String,
        tags: Vec<String>,
    },
}

// =============================================================================
// Static Test Data Tables
// =============================================================================

/// Server version test cases
pub const VERSION_TEST_CASES: &[VersionTestCase] = &[
    VersionTestCase {
        function_name: "PnL",
        required_version: server_versions::PNL,
    },
    VersionTestCase {
        function_name: "PnL Single",
        required_version: server_versions::REALIZED_PNL,
    },
    VersionTestCase {
        function_name: "Account Summary",
        required_version: server_versions::ACCOUNT_SUMMARY,
    },
    VersionTestCase {
        function_name: "Positions Multi",
        required_version: server_versions::MODELS_SUPPORT,
    },
    VersionTestCase {
        function_name: "Account Updates Multi",
        required_version: server_versions::MODELS_SUPPORT,
    },
    VersionTestCase {
        function_name: "Family Codes",
        required_version: server_versions::REQ_FAMILY_CODES,
    },
    VersionTestCase {
        function_name: "Positions",
        required_version: server_versions::POSITIONS,
    },
];

// =============================================================================
// Dynamic Test Data Functions
// =============================================================================

/// Managed accounts test cases
pub fn managed_accounts_test_cases() -> Vec<ManagedAccountsTestCase> {
    vec![
        ManagedAccountsTestCase {
            scenario: "valid multiple accounts",
            responses: vec!["17|1|DU1234567,DU7654321|".into()],
            expected: vec!["DU1234567".to_string(), "DU7654321".to_string()],
            description: "Multiple comma-separated accounts",
        },
        ManagedAccountsTestCase {
            scenario: "single account",
            responses: vec!["17|1|SINGLE_ACCOUNT|".into()],
            expected: vec!["SINGLE_ACCOUNT".to_string()],
            description: "Single account response",
        },
        ManagedAccountsTestCase {
            scenario: "empty response",
            responses: vec!["17|1||".into()],
            expected: vec![],
            description: "Empty account string results in empty vector",
        },
        ManagedAccountsTestCase {
            scenario: "no response",
            responses: vec![],
            expected: vec![],
            description: "No message received returns empty vector",
        },
        ManagedAccountsTestCase {
            scenario: "accounts with trailing comma",
            responses: vec!["17|1|ACC1,ACC2,|".into()],
            expected: vec!["ACC1".to_string(), "ACC2".to_string()],
            description: "Trailing comma is ignored",
        },
    ]
}

/// Server time test cases  
pub fn server_time_test_cases() -> Vec<ServerTimeTestCase> {
    vec![
        ServerTimeTestCase {
            scenario: "valid timestamp",
            responses: vec!["49|1|1678890000|".into()], // 2023-03-15 14:20:00 UTC
            expected_result: Ok(datetime!(2023-03-15 14:20:00 UTC)),
            expected_request: "49|1|",
        },
        ServerTimeTestCase {
            scenario: "unix epoch",
            responses: vec!["49|1|0|".into()],
            expected_result: Ok(datetime!(1970-01-01 0:00 UTC)),
            expected_request: "49|1|",
        },
        ServerTimeTestCase {
            scenario: "y2k timestamp",
            responses: vec!["49|1|946684800|".into()],
            expected_result: Ok(datetime!(2000-01-01 0:00 UTC)),
            expected_request: "49|1|",
        },
        ServerTimeTestCase {
            scenario: "invalid timestamp string",
            responses: vec!["49|1|invalid_timestamp|".into()],
            expected_result: Err("Parse/ParseInt/Simple error expected"),
            expected_request: "49|1|",
        },
        ServerTimeTestCase {
            scenario: "overflow timestamp",
            responses: vec!["49|1|99999999999999999999|".into()],
            expected_result: Err("Parse/ParseInt/Simple error expected"),
            expected_request: "49|1|",
        },
        ServerTimeTestCase {
            scenario: "no response",
            responses: vec![],
            expected_result: Err("No response from server"),
            expected_request: "49|1|",
        },
    ]
}

/// Contract ID edge case test cases
pub fn contract_id_test_cases() -> Vec<ContractIdTestCase> {
    vec![
        ContractIdTestCase {
            description: "standard contract ID",
            contract_id: ContractId(1001),
            expected_pattern: "|1001|".to_string(),
        },
        ContractIdTestCase {
            description: "zero contract ID",
            contract_id: ContractId(0),
            expected_pattern: "|0|".to_string(),
        },
        ContractIdTestCase {
            description: "max contract ID",
            contract_id: ContractId(i32::MAX),
            expected_pattern: format!("|{}|", i32::MAX),
        },
        ContractIdTestCase {
            description: "negative contract ID",
            contract_id: ContractId(-1),
            expected_pattern: "|-1|".to_string(),
        },
        ContractIdTestCase {
            description: "large positive ID",
            contract_id: ContractId(999999999),
            expected_pattern: "|999999999|".to_string(),
        },
    ]
}

/// PnL parameter combination test cases
pub fn pnl_parameter_test_cases() -> Vec<PnLTestCase> {
    vec![
        PnLTestCase {
            description: "PnL with TARGET2024 model",
            model_code: Some("TARGET2024".to_string()),
            expected_pattern: "TARGET2024",
        },
        PnLTestCase {
            description: "PnL with MODEL1",
            model_code: Some("MODEL1".to_string()),
            expected_pattern: "MODEL1",
        },
        PnLTestCase {
            description: "PnL with MODEL2",
            model_code: Some("MODEL2".to_string()),
            expected_pattern: "MODEL2",
        },
        PnLTestCase {
            description: "PnL with no model code",
            model_code: None,
            expected_pattern: "||",
        },
        PnLTestCase {
            description: "PnL with empty model code",
            model_code: Some("".to_string()),
            expected_pattern: "||",
        },
    ]
}

/// Positions multi parameter combination test cases
pub fn positions_multi_parameter_test_cases() -> Vec<PositionsMultiTestCase> {
    vec![
        PositionsMultiTestCase {
            description: "both account and model",
            account: Some("DU1234567".to_string()),
            model_code: Some("TARGET2024".to_string()),
        },
        PositionsMultiTestCase {
            description: "account only",
            account: Some("DU1234567".to_string()),
            model_code: None,
        },
        PositionsMultiTestCase {
            description: "model only",
            account: None,
            model_code: Some("TARGET2024".to_string()),
        },
        PositionsMultiTestCase {
            description: "neither account nor model",
            account: None,
            model_code: None,
        },
        PositionsMultiTestCase {
            description: "different account",
            account: Some("DU7654321".to_string()),
            model_code: Some("PROD2024".to_string()),
        },
    ]
}

/// Account summary tag combination test cases
pub fn account_summary_tag_test_cases() -> Vec<AccountSummaryTagTestCase> {
    vec![
        AccountSummaryTagTestCase {
            description: "multiple standard tags",
            group: "All".to_string(),
            tags: vec!["AccountType", "NetLiquidation", "TotalCashValue"],
            expected_tag_encoding: Some("AccountType,NetLiquidation,TotalCashValue"),
            should_succeed: true,
            expect_responses: true,
        },
        AccountSummaryTagTestCase {
            description: "single tag",
            group: "All".to_string(),
            tags: vec!["AccountType"],
            expected_tag_encoding: Some("AccountType"),
            should_succeed: true,
            expect_responses: true,
        },
        AccountSummaryTagTestCase {
            description: "empty tags list",
            group: "All".to_string(),
            tags: vec![],
            expected_tag_encoding: Some(""),
            should_succeed: true,
            expect_responses: false,
        },
        AccountSummaryTagTestCase {
            description: "all available tags",
            group: "All".to_string(),
            tags: vec![
                "AccountType", "NetLiquidation", "TotalCashValue", "SettledCash", 
                "AccruedCash", "BuyingPower", "EquityWithLoanValue", "PreviousEquityWithLoanValue",
                "GrossPositionValue", "RegTEquity", "RegTMargin", "SMA"
            ],
            expected_tag_encoding: Some("AccountType,NetLiquidation,TotalCashValue,SettledCash,AccruedCash,BuyingPower,EquityWithLoanValue,PreviousEquityWithLoanValue,GrossPositionValue,RegTEquity,RegTMargin,SMA"),
            should_succeed: true,
            expect_responses: true,
        },
        AccountSummaryTagTestCase {
            description: "family group",
            group: "Family".to_string(),
            tags: vec!["AccountType", "NetLiquidation"],
            expected_tag_encoding: Some("AccountType,NetLiquidation"),
            should_succeed: true,
            expect_responses: true,
        },
        AccountSummaryTagTestCase {
            description: "custom group with single tag",
            group: "MyGroup".to_string(),
            tags: vec!["TotalCashValue"],
            expected_tag_encoding: Some("TotalCashValue"),
            should_succeed: true,
            expect_responses: true,
        },
    ]
}

/// Subscription lifecycle test cases
pub fn subscription_lifecycle_test_cases() -> Vec<SubscriptionLifecycleTestCase> {
    vec![
        SubscriptionLifecycleTestCase {
            description: "PnL subscription with model code",
            subscription_type: SubscriptionType::PnL {
                account: "DU1234567".to_string(),
                model_code: Some("TARGET2024".to_string()),
            },
            expected_subscribe_pattern: "92|",
            expected_cancel_pattern: "93|",
        },
        SubscriptionLifecycleTestCase {
            description: "PnL subscription without model code",
            subscription_type: SubscriptionType::PnL {
                account: "DU1234567".to_string(),
                model_code: None,
            },
            expected_subscribe_pattern: "92|",
            expected_cancel_pattern: "93|",
        },
        SubscriptionLifecycleTestCase {
            description: "Positions subscription",
            subscription_type: SubscriptionType::Positions,
            expected_subscribe_pattern: "61|",
            expected_cancel_pattern: "64|",
        },
        SubscriptionLifecycleTestCase {
            description: "Account Summary subscription",
            subscription_type: SubscriptionType::AccountSummary {
                group: "All".to_string(),
                tags: vec!["AccountType".to_string()],
            },
            expected_subscribe_pattern: "62|",
            expected_cancel_pattern: "63|",
        },
        SubscriptionLifecycleTestCase {
            description: "Positions Multi subscription",
            subscription_type: SubscriptionType::PositionsMulti {
                account: Some("DU1234567".to_string()),
                model_code: Some("TARGET2024".to_string()),
            },
            expected_subscribe_pattern: "74|",
            expected_cancel_pattern: "75|",
        },
        SubscriptionLifecycleTestCase {
            description: "PnL Single subscription",
            subscription_type: SubscriptionType::PnLSingle {
                account: "DU1234567".to_string(),
                contract_id: 1001,
                model_code: Some("TARGET2024".to_string()),
            },
            expected_subscribe_pattern: "94|",
            expected_cancel_pattern: "95|",
        },
    ]
}
