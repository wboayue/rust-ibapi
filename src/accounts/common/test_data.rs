//! Shared test data tables for both sync and async test implementations

#[cfg(test)]
pub(in crate::accounts) mod tables {
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

    /// Managed accounts test cases
    pub fn managed_accounts_test_cases() -> Vec<ManagedAccountsTestCase> {
        vec![
            ManagedAccountsTestCase {
                scenario: "empty response",
                responses: vec!["15|1||".into()],
                expected: vec![String::new()],
                description: "Expected single empty account",
            },
            ManagedAccountsTestCase {
                scenario: "no response",
                responses: vec![],
                expected: vec![],
                description: "Expected empty accounts list",
            },
            ManagedAccountsTestCase {
                scenario: "single account",
                responses: vec!["15|1|SINGLE_ACCOUNT|".into()],
                expected: vec!["SINGLE_ACCOUNT".to_string()],
                description: "Expected single account",
            },
            ManagedAccountsTestCase {
                scenario: "multiple accounts with trailing comma",
                responses: vec!["15|1|ACC1,ACC2,|".into()],
                expected: vec!["ACC1".to_string(), "ACC2".to_string(), String::new()],
                description: "Expected accounts with empty trailing entry",
            },
        ]
    }

    /// Server time test cases  
    pub fn server_time_test_cases() -> Vec<ServerTimeTestCase> {
        vec![
            ServerTimeTestCase {
                scenario: "valid timestamp",
                responses: vec![format!("49|1|1678890000|")], // 2023-03-15 14:20:00 UTC
                expected_result: Ok(datetime!(2023-03-15 14:20:00 UTC)),
                expected_request: "49|1|",
            },
            ServerTimeTestCase {
                scenario: "unix epoch",
                responses: vec![format!("49|1|0|")],
                expected_result: Ok(datetime!(1970-01-01 0:00 UTC)),
                expected_request: "49|1|",
            },
            ServerTimeTestCase {
                scenario: "y2k timestamp",
                responses: vec![format!("49|1|946684800|")],
                expected_result: Ok(datetime!(2000-01-01 0:00 UTC)),
                expected_request: "49|1|",
            },
            ServerTimeTestCase {
                scenario: "invalid timestamp string",
                responses: vec![format!("49|1|invalid_timestamp|")],
                expected_result: Err("Parse/ParseInt/Simple error expected"),
                expected_request: "49|1|",
            },
            ServerTimeTestCase {
                scenario: "overflow timestamp",
                responses: vec![format!("49|1|99999999999999999999|")],
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
}
