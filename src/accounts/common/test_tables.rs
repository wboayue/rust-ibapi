//! Table-driven test data for accounts module tests

#[allow(unused_imports)]
use crate::accounts::types::*;

/// Test case for contract ID edge cases
#[derive(Debug, Clone)]
pub struct ContractIdTestCase {
    pub description: &'static str,
    pub contract_id: ContractId,
}

/// Test case for PnL parameter combinations
#[derive(Debug, Clone)]
pub struct PnLTestCase {
    pub description: &'static str,
    pub model_code: Option<String>,
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
    #[cfg_attr(not(feature = "async"), allow(dead_code))]
    pub expected_tag_encoding: Option<&'static str>,
    #[cfg_attr(not(feature = "sync"), allow(dead_code))]
    pub should_succeed: bool,
    pub expect_responses: bool,
}

/// Test case for subscription lifecycle testing
#[derive(Debug, Clone)]
pub struct SubscriptionLifecycleTestCase {
    pub description: &'static str,
    pub subscription_type: SubscriptionType,
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
// Dynamic Test Data Functions
// =============================================================================

/// Contract ID edge case test cases
pub fn contract_id_test_cases() -> Vec<ContractIdTestCase> {
    vec![
        ContractIdTestCase {
            description: "standard contract ID",
            contract_id: ContractId(1001),
        },
        ContractIdTestCase {
            description: "zero contract ID",
            contract_id: ContractId(0),
        },
        ContractIdTestCase {
            description: "max contract ID",
            contract_id: ContractId(i32::MAX),
        },
        ContractIdTestCase {
            description: "negative contract ID",
            contract_id: ContractId(-1),
        },
        ContractIdTestCase {
            description: "large positive ID",
            contract_id: ContractId(999999999),
        },
    ]
}

/// PnL parameter combination test cases
pub fn pnl_parameter_test_cases() -> Vec<PnLTestCase> {
    vec![
        PnLTestCase {
            description: "PnL with TARGET2024 model",
            model_code: Some("TARGET2024".to_string()),
        },
        PnLTestCase {
            description: "PnL with MODEL1",
            model_code: Some("MODEL1".to_string()),
        },
        PnLTestCase {
            description: "PnL with MODEL2",
            model_code: Some("MODEL2".to_string()),
        },
        PnLTestCase {
            description: "PnL with no model code",
            model_code: None,
        },
        PnLTestCase {
            description: "PnL with empty model code",
            model_code: Some("".to_string()),
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
        },
        SubscriptionLifecycleTestCase {
            description: "PnL subscription without model code",
            subscription_type: SubscriptionType::PnL {
                account: "DU1234567".to_string(),
                model_code: None,
            },
        },
        SubscriptionLifecycleTestCase {
            description: "Positions subscription",
            subscription_type: SubscriptionType::Positions,
        },
        SubscriptionLifecycleTestCase {
            description: "Account Summary subscription",
            subscription_type: SubscriptionType::AccountSummary {
                group: "All".to_string(),
                tags: vec!["AccountType".to_string()],
            },
        },
        SubscriptionLifecycleTestCase {
            description: "Positions Multi subscription",
            subscription_type: SubscriptionType::PositionsMulti {
                account: Some("DU1234567".to_string()),
                model_code: Some("TARGET2024".to_string()),
            },
        },
        SubscriptionLifecycleTestCase {
            description: "PnL Single subscription",
            subscription_type: SubscriptionType::PnLSingle {
                account: "DU1234567".to_string(),
                contract_id: 1001,
                model_code: Some("TARGET2024".to_string()),
            },
        },
    ]
}
