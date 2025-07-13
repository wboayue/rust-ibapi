//! Table-driven test data for contracts module tests

use crate::contracts::{Contract, ContractDetails, SecurityType};
use crate::messages::OutgoingMessages;
use crate::server_versions;

/// Test case for contract details tests
pub struct ContractDetailsTestCase {
    pub name: &'static str,
    pub contract: Contract,
    pub response_messages: Vec<String>,
    pub expected_request: &'static str,
    pub expected_count: usize,
    pub validations: Box<dyn Fn(&[ContractDetails]) + Send + Sync>,
}

/// Test case for matching symbols tests
pub struct MatchingSymbolsTestCase {
    pub name: &'static str,
    pub pattern: &'static str,
    pub response_message: String,
    pub expected_request: &'static str,
    pub expected_count: usize,
}

/// Test case for market rule tests
pub struct MarketRuleTestCase {
    pub name: &'static str,
    pub market_rule_id: i32,
    pub response_message: String,
    pub expected_request: &'static str,
    pub expected_price_increments: usize,
}

/// Test case for option calculation tests
pub struct OptionCalculationTestCase {
    pub name: &'static str,
    pub contract: Contract,
    pub volatility: Option<f64>,
    pub option_price: Option<f64>,
    pub underlying_price: f64,
    pub response_message: String,
    pub expected_request_prefix: &'static str,
    pub expected_price: f64,
    pub expected_delta: f64,
}

/// Test case for option chain tests
pub struct OptionChainTestCase {
    pub name: &'static str,
    pub symbol: &'static str,
    pub exchange: &'static str,
    pub security_type: SecurityType,
    pub contract_id: i32,
    pub response_messages: Vec<String>,
    pub expected_request: &'static str,
    pub expected_count: usize,
}

/// Test case for verify contract tests
pub struct VerifyContractTestCase {
    pub name: &'static str,
    pub contract: Contract,
    pub server_version: i32,
    pub should_error: bool,
    pub error_contains: Option<&'static str>,
}

/// Test case for StreamDecoder tests
pub struct StreamDecoderTestCase {
    pub name: &'static str,
    pub message: &'static str,
    pub expected_result: StreamDecoderResult,
}

pub enum StreamDecoderResult {
    OptionComputation { price: f64, delta: f64 },
    OptionChain { exchange: String, underlying_conid: i32 },
    Error(&'static str),
}

/// Test case for cancel message generation
pub struct CancelMessageTestCase {
    pub name: &'static str,
    pub decoder_type: &'static str,
    pub request_id: Option<i32>,
    pub request_type: Option<OutgoingMessages>,
    pub expected_message: Result<String, &'static str>,
}

/// Test cases for contract details
pub fn contract_details_test_cases() -> Vec<ContractDetailsTestCase> {
    vec![
        ContractDetailsTestCase {
            name: "stock contract details",
            contract: Contract {
                symbol: "TSLA".to_string(),
                security_type: SecurityType::Stock,
                exchange: "SMART".to_string(),
                currency: "USD".to_string(),
                ..Default::default()
            },
            response_messages: vec![
                "10\09001\0TSLA\0STK\0\00\0\0SMART\0USD\0TSLA\0NMS\0NMS\0459200101\00.01\0\0ACTIVETIM,AD,ADJUST,ALERT,ALGO,ALLOC,AON,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DARKONLY,DARKPOLL,DAY,DEACT,DEACTDIS,DEACTEOD,DIS,DUR,GAT,GTC,GTD,GTT,HID,IBKRATS,ICE,IMB,IOC,LIT,LMT,LOC,MIDPX,MIT,MKT,MOC,MTL,NGCOMB,NODARK,NONALGO,OCA,OPG,OPGREROUT,PEGBENCH,PEGMID,POSTATS,POSTONLY,PREOPGRTH,PRICECHK,REL,REL2MID,RELPCTOFS,RPI,RTH,SCALE,SCALEODD,SCALERST,SIZECHK,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,SWEEP,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF\0SMART,AMEX,NYSE,CBOE,PHLX,ISE,CHX,ARCA,ISLAND,DRCTEDGE,BEX,BATS,EDGEA,CSFBALGO,JEFFALGO,BYX,IEX,EDGX,FOXRIVER,PEARL,NYSENAT,LTSE,MEMX,PSX\01\00\0TESLA INC\0NASDAQ\0\0Consumer, Cyclical\0Auto Manufacturers\0Auto-Cars/Light Trucks\0US/Eastern\020221229:0400-20221229:2000;20221230:0400-20221230:2000;20221231:CLOSED;20230101:CLOSED;20230102:CLOSED;20230103:0400-20230103:2000\020221229:0930-20221229:1600;20221230:0930-20221230:1600;20221231:CLOSED;20230101:CLOSED;20230102:CLOSED;20230103:0930-20230103:1600\0\00\01\0ISIN\0US88160R1014\01\0\0\026,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26,26\0\0COMMON\01\01\0100\0\0".to_string(),
                "52\01\09001\0\0".to_string(),
            ],
            expected_request: "9|8|9000|0|TSLA|STK||0|||SMART||USD|||0|||",
            expected_count: 1,
            validations: Box::new(|contracts| {
                assert_eq!(contracts[0].contract.symbol, "TSLA");
                assert_eq!(contracts[0].contract.security_type, SecurityType::Stock);
                assert_eq!(contracts[0].contract.currency, "USD");
                assert_eq!(contracts[0].contract.contract_id, 459200101);
                assert_eq!(contracts[0].long_name, "TESLA INC");
                assert_eq!(contracts[0].industry, "Consumer, Cyclical");
                assert_eq!(contracts[0].category, "Auto Manufacturers");
                assert_eq!(contracts[0].stock_type, "COMMON");
                assert_eq!(contracts[0].min_size, 1.0);
                assert_eq!(contracts[0].size_increment, 1.0);
                assert_eq!(contracts[0].suggested_size_increment, 100.0);
            }),
        },
        ContractDetailsTestCase {
            name: "future contract details",
            contract: Contract {
                symbol: "ES".to_string(),
                security_type: SecurityType::Future,
                exchange: "CME".to_string(),
                last_trade_date_or_contract_month: "202406".to_string(),
                currency: "USD".to_string(),
                ..Default::default()
            },
            response_messages: vec![
                "10\09001\0ES\0FUT\020240621\00\0\0CME\0USD\0ESM4\0CME\0CME\0551318584\00.25\050\0ACTIVETIM,AD,ADJUST,ALERT,ALGO,ALLOC,AON,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DAY,DEACT,DEACTDIS,DEACTEOD,GAT,GTC,GTD,GTT,HID,ICE,IOC,LIT,LMT,LOC,MIT,MKT,MOC,MTL,NGCOMB,NONALGO,OCA,PEGBENCH,PEGMID,PEGSTK,POSTONLY,PREOPGRTH,REL,RPI,RTH,SCALE,SCALEODD,SCALERST,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF\0CME\01\00\0E-mini S&P 500\0CME\0202406\0Financial\0Indices\0Broad Market Equity Index\0US/Central\020240617:CLOSED;20240618:1700-20240618:1600;20240619:1700-20240619:1600;20240620:1700-20240620:1600;20240621:1700-20240621:0900\020240617:CLOSED;20240618:1700-20240618:1600;20240619:1700-20240619:1600;20240620:1700-20240620:1600;20240621:1700-20240621:0900\0\00\01\0\0\01\0\0\0\0\0FUT\01\01\01\00.25\0\0".to_string(),
                "52\01\09001\0\0".to_string(),
            ],
            expected_request: "9|8|9000|0|ES|FUT|202406|0|||CME||USD|||0|||",
            expected_count: 1,
            validations: Box::new(|contracts| {
                assert_eq!(contracts[0].contract.symbol, "ES");
                assert_eq!(contracts[0].contract.security_type, SecurityType::Future);
                assert_eq!(contracts[0].contract.exchange, "CME");
                assert_eq!(contracts[0].contract.contract_id, 551318584);
                assert_eq!(contracts[0].contract.multiplier, "50");
                assert_eq!(contracts[0].contract.last_trade_date_or_contract_month, "20240621");
                assert_eq!(contracts[0].long_name, "E-mini S&P 500");
                assert_eq!(contracts[0].category, "Indices");
            }),
        },
    ]
}

/// Test cases for matching symbols
pub fn matching_symbols_test_cases() -> Vec<MatchingSymbolsTestCase> {
    vec![
        MatchingSymbolsTestCase {
            name: "single match",
            pattern: "AAPL",
            response_message: "79\09000\01\012345\0AAPL\0STK\0NASDAQ\0USD\00\0APPLE INC\0\0".to_string(),
            expected_request: "81|9000|AAPL|",
            expected_count: 1,
        },
        MatchingSymbolsTestCase {
            name: "multiple matches",
            pattern: "AA",
            response_message: "79\09000\02\067890\0AA\0STK\0SMART\0USD\01\0OPT\0ALCOA CORP\0\012346\0AAPL\0STK\0SMART\0USD\00\0APPLE INC\0\0"
                .to_string(),
            expected_request: "81|9000|AA|",
            expected_count: 2,
        },
    ]
}

/// Test cases for market rule
pub fn market_rule_test_cases() -> Vec<MarketRuleTestCase> {
    vec![
        MarketRuleTestCase {
            name: "standard market rule",
            market_rule_id: 26,
            response_message: "87\026\04\00\00.01\00.01\00.01\01\01\05\00.05\0".to_string(),
            expected_request: "91|26|",
            expected_price_increments: 4,
        },
        MarketRuleTestCase {
            name: "complex market rule",
            market_rule_id: 635,
            response_message: "87\0635\03\00\00.0001\00.01\00.001\010\00.01\0".to_string(),
            expected_request: "91|635|",
            expected_price_increments: 3,
        },
    ]
}

/// Test cases for option calculations
pub fn option_calculation_test_cases() -> Vec<OptionCalculationTestCase> {
    vec![
        OptionCalculationTestCase {
            name: "calculate option price",
            contract: Contract {
                symbol: "AAPL".to_string(),
                security_type: SecurityType::Option,
                last_trade_date_or_contract_month: "20231215".to_string(),
                strike: 150.0,
                right: "C".to_string(),
                exchange: "SMART".to_string(),
                currency: "USD".to_string(),
                multiplier: "100".to_string(),
                ..Default::default()
            },
            volatility: Some(0.3),
            option_price: None,
            underlying_price: 145.0,
            response_message: "21\06\09000\013\00.3\00.42\07.85\0-0.03\00.65\0-0.002\00.98\06.87\0145.0\07.85\0".to_string(),
            expected_request_prefix: "54|3|9000|0|AAPL|OPT|20231215|150|C|100|SMART||USD||0.3|145|",
            expected_price: 7.85,
            expected_delta: 0.42,
        },
        OptionCalculationTestCase {
            name: "calculate implied volatility",
            contract: Contract {
                symbol: "AAPL".to_string(),
                security_type: SecurityType::Option,
                last_trade_date_or_contract_month: "20231215".to_string(),
                strike: 150.0,
                right: "C".to_string(),
                exchange: "SMART".to_string(),
                currency: "USD".to_string(),
                multiplier: "100".to_string(),
                ..Default::default()
            },
            volatility: None,
            option_price: Some(5.0),
            underlying_price: 145.0,
            response_message: "21\06\09000\013\00.25\00.32\05.0\0-0.02\00.45\0-0.001\00.25\04.55\0145.0\05.0\0".to_string(),
            expected_request_prefix: "54|3|9000|0|AAPL|OPT|20231215|150|C|100|SMART||USD||5|145|",
            expected_price: 5.0,
            expected_delta: 0.32,
        },
    ]
}

/// Test cases for option chain
pub fn option_chain_test_cases() -> Vec<OptionChainTestCase> {
    vec![
        OptionChainTestCase {
            name: "stock option chain",
            symbol: "AAPL",
            exchange: "SMART",
            security_type: SecurityType::Stock,
            contract_id: 0,
            response_messages: vec![
                "75\09000\0SMART\0265598\0100\00\012\020230120\020230217\020230317\020230421\020230519\020230616\020230721\020230818\020231215\020240119\020240621\020250117\031\050\055\060\065\070\075\080\085\090\095\0100\0105\0110\0115\0120\0125\0130\0135\0140\0145\0150\0155\0160\0165\0170\0175\0180\0185\0190\0195\0200\0".to_string(),
                "76\09000\0".to_string(),
            ],
            expected_request: "78|9000|AAPL|SMART|STK|0|",
            expected_count: 1,
        },
    ]
}

/// Test cases for verify contract
pub fn verify_contract_test_cases() -> Vec<VerifyContractTestCase> {
    vec![
        VerifyContractTestCase {
            name: "valid contract",
            contract: Contract {
                symbol: "AAPL".to_string(),
                security_type: SecurityType::Stock,
                exchange: "SMART".to_string(),
                currency: "USD".to_string(),
                ..Default::default()
            },
            server_version: server_versions::SIZE_RULES,
            should_error: false,
            error_contains: None,
        },
        VerifyContractTestCase {
            name: "contract with security_id - old server",
            contract: Contract {
                symbol: "AAPL".to_string(),
                security_type: SecurityType::Stock,
                exchange: "SMART".to_string(),
                currency: "USD".to_string(),
                security_id: "US0378331005".to_string(),
                security_id_type: "ISIN".to_string(),
                ..Default::default()
            },
            server_version: server_versions::SEC_ID_TYPE - 1,
            should_error: true,
            error_contains: Some("security ID type"),
        },
        VerifyContractTestCase {
            name: "contract with trading class - old server",
            contract: Contract {
                symbol: "AAPL".to_string(),
                security_type: SecurityType::Stock,
                exchange: "SMART".to_string(),
                currency: "USD".to_string(),
                trading_class: "AAPL".to_string(),
                ..Default::default()
            },
            server_version: server_versions::TRADING_CLASS - 1,
            should_error: true,
            error_contains: Some("trading class"),
        },
    ]
}

/// Test cases for StreamDecoder implementations
pub fn stream_decoder_test_cases() -> Vec<StreamDecoderTestCase> {
    vec![
        StreamDecoderTestCase {
            name: "valid option computation",
            message: "21\06\09000\013\00.3\00.35\05.25\0-0.025\00.55\0-0.0015\00.3\04.75\0140.0\05.25\0",
            expected_result: StreamDecoderResult::OptionComputation { price: 5.25, delta: 0.35 },
        },
        StreamDecoderTestCase {
            name: "valid option chain",
            message: "75\09000\0SMART\0265598\0100\00\02\020230120\020230217\03\050\055\060\0",
            expected_result: StreamDecoderResult::OptionChain {
                exchange: "SMART".to_string(),
                underlying_conid: 265598,
            },
        },
        StreamDecoderTestCase {
            name: "option chain end of stream",
            message: "76\09000\0",
            expected_result: StreamDecoderResult::Error("EndOfStream"),
        },
        StreamDecoderTestCase {
            name: "unexpected message type",
            message: "1\09000\0unexpected\0",
            expected_result: StreamDecoderResult::Error("UnexpectedResponse"),
        },
    ]
}

/// Test cases for cancel message generation
pub fn cancel_message_test_cases() -> Vec<CancelMessageTestCase> {
    vec![
        CancelMessageTestCase {
            name: "cancel implied volatility calculation",
            decoder_type: "OptionComputation",
            request_id: Some(9000),
            request_type: Some(OutgoingMessages::ReqCalcImpliedVolat),
            expected_message: Ok("56|1|9000|".to_string()), // CancelImpliedVolatility
        },
        CancelMessageTestCase {
            name: "cancel option price calculation",
            decoder_type: "OptionComputation",
            request_id: Some(9001),
            request_type: Some(OutgoingMessages::ReqCalcOptionPrice),
            expected_message: Ok("57|1|9001|".to_string()), // CancelOptionPrice
        },
        CancelMessageTestCase {
            name: "cancel option chain - not supported",
            decoder_type: "OptionChain",
            request_id: Some(9003),
            request_type: None,
            expected_message: Err("cancel not implemented"),
        },
    ]
}
