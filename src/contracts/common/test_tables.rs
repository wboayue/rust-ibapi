//! Table-driven test data for contracts module tests

#[cfg(feature = "sync")]
use crate::common::test_utils::helpers::proto_error_response;
use crate::common::test_utils::helpers::{proto_response, text_response};
use crate::contracts::{Contract, ContractDetails, Currency, Exchange, OptionRight, SecurityIdType, SecurityType, Symbol};
use crate::messages::{IncomingMessages, OutgoingMessages, ResponseMessage};
use crate::server_versions;
use crate::testdata::builders::contracts::{contract_data, market_rule, option_chain, smart_components, symbol_samples, symbol_samples_entry};
use crate::testdata::builders::market_data::tick_option_computation;
use crate::testdata::builders::ResponseProtoEncoder;

/// Test case for contract details tests
#[allow(clippy::type_complexity, dead_code)]
pub struct ContractDetailsTestCase {
    pub name: &'static str,
    pub contract: Contract,
    pub ordered_responses: Vec<ResponseMessage>,
    pub expected_request: &'static str,
    pub expected_count: usize,
    pub validations: Box<dyn Fn(&[ContractDetails]) + Send + Sync>,
}

/// Test case for matching symbols tests
#[allow(dead_code)]
pub struct MatchingSymbolsTestCase {
    pub name: &'static str,
    pub pattern: &'static str,
    pub ordered_responses: Vec<ResponseMessage>,
    pub expected_request: &'static str,
    pub expected_count: usize,
}

/// Test case for market rule tests
#[allow(dead_code)]
pub struct MarketRuleTestCase {
    pub name: &'static str,
    pub market_rule_id: i32,
    pub ordered_responses: Vec<ResponseMessage>,
    pub expected_request: &'static str,
    pub expected_price_increments: usize,
}

/// Test case for smart components tests
#[allow(dead_code)]
pub struct SmartComponentsTestCase {
    pub name: &'static str,
    pub bbo_exchange: &'static str,
    pub ordered_responses: Vec<ResponseMessage>,
    pub expected_count: usize,
    pub expected_first: Option<(i32, &'static str, &'static str)>,
}

/// Test case for option calculation tests
#[allow(dead_code)]
pub struct OptionCalculationTestCase {
    pub name: &'static str,
    pub contract: Contract,
    pub volatility: Option<f64>,
    pub option_price: Option<f64>,
    pub underlying_price: f64,
    pub ordered_responses: Vec<ResponseMessage>,
    pub expected_request_prefix: &'static str,
    pub expected_price: f64,
    pub expected_delta: f64,
}

/// Test case for option chain tests
#[allow(dead_code)]
pub struct OptionChainTestCase {
    pub name: &'static str,
    pub symbol: &'static str,
    pub exchange: &'static str,
    pub security_type: SecurityType,
    pub contract_id: i32,
    pub ordered_responses: Vec<ResponseMessage>,
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
    pub message: ResponseMessage,
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
    pub expected_msg_id: Result<i32, &'static str>,
}

const STK_ORDER_TYPES: &str = "ACTIVETIM,AD,ADJUST,ALERT,ALGO,ALLOC,AON,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DARKONLY,DARKPOLL,DAY,DEACT,DEACTDIS,DEACTEOD,DIS,DUR,GAT,GTC,GTD,GTT,HID,IBKRATS,ICE,IMB,IOC,LIT,LMT,LOC,MIDPX,MIT,MKT,MOC,MTL,NGCOMB,NODARK,NONALGO,OCA,OPG,OPGREROUT,PEGBENCH,PEGMID,POSTATS,POSTONLY,PREOPGRTH,PRICECHK,REL,REL2MID,RELPCTOFS,RPI,RTH,SCALE,SCALEODD,SCALERST,SIZECHK,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,SWEEP,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF";
const STK_VALID_EXCHANGES: &str =
    "SMART,AMEX,NYSE,CBOE,PHLX,ISE,CHX,ARCA,ISLAND,DRCTEDGE,BEX,BATS,EDGEA,CSFBALGO,JEFFALGO,BYX,IEX,EDGX,FOXRIVER,PEARL,NYSENAT,LTSE,MEMX,PSX";
const FUT_ORDER_TYPES: &str = "ACTIVETIM,AD,ADJUST,ALERT,ALGO,ALLOC,AON,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DAY,DEACT,DEACTDIS,DEACTEOD,GAT,GTC,GTD,GTT,HID,ICE,IOC,LIT,LMT,LOC,MIT,MKT,MOC,MTL,NGCOMB,NONALGO,OCA,PEGBENCH,PEGMID,PEGSTK,POSTONLY,PREOPGRTH,REL,RPI,RTH,SCALE,SCALEODD,SCALERST,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF";
const AMEX_ORDER_TYPES: &str = "ACTIVETIM,AD,ADJUST,ALERT,ALLOC,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DAY,DEACT,DEACTDIS,DEACTEOD,GAT,GTC,GTD,GTT,HID,IOC,LIT,LMT,MIT,MKT,MTL,NGCOMB,NONALGO,OCA,PEGBENCH,SCALE,SCALERST,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF";

fn contract_data_end(request_id: i32) -> ResponseMessage {
    text_response(format!("52|1|{request_id}|"))
}

/// Test cases for contract details
pub fn contract_details_test_cases() -> Vec<ContractDetailsTestCase> {
    vec![
        ContractDetailsTestCase {
            name: "stock contract details",
            contract: Contract {
                symbol: Symbol::from("TSLA"),
                security_type: SecurityType::Stock,
                exchange: Exchange::from("SMART"),
                currency: Currency::from("USD"),
                ..Default::default()
            },
            ordered_responses: vec![
                proto_response(
                    IncomingMessages::ContractData,
                    contract_data()
                        .request_id(9001)
                        .contract_id(459200101)
                        .symbol("TSLA")
                        .security_type("STK")
                        .exchange("SMART")
                        .currency("USD")
                        .local_symbol("TSLA")
                        .trading_class("NMS")
                        .market_name("NMS")
                        .order_types(STK_ORDER_TYPES)
                        .valid_exchanges(STK_VALID_EXCHANGES)
                        .long_name("TESLA INC")
                        .primary_exchange("NASDAQ")
                        .industry("Consumer, Cyclical")
                        .category("Auto Manufacturers")
                        .subcategory("Auto-Cars/Light Trucks")
                        .time_zone_id("US/Eastern")
                        .stock_type("COMMON")
                        .encode_proto(),
                ),
                contract_data_end(9001),
            ],
            expected_request: "9|8|9000|0|TSLA|STK||0|||SMART||USD|||0|||",
            expected_count: 1,
            validations: Box::new(|contracts| {
                assert_eq!(contracts[0].contract.symbol, Symbol::from("TSLA"));
                assert_eq!(contracts[0].contract.security_type, SecurityType::Stock);
                assert_eq!(contracts[0].contract.currency, Currency::from("USD"));
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
                symbol: Symbol::from("ES"),
                security_type: SecurityType::Future,
                exchange: Exchange::from("CME"),
                last_trade_date_or_contract_month: "202406".to_string(),
                currency: Currency::from("USD"),
                ..Default::default()
            },
            ordered_responses: vec![
                proto_response(
                    IncomingMessages::ContractData,
                    contract_data()
                        .request_id(9001)
                        .contract_id(551318584)
                        .symbol("ES")
                        .security_type("FUT")
                        .last_trade_date_or_contract_month("20240621")
                        .multiplier("50")
                        .exchange("CME")
                        .currency("USD")
                        .local_symbol("ESM4")
                        .trading_class("CME")
                        .market_name("CME")
                        .order_types(FUT_ORDER_TYPES)
                        .valid_exchanges("CME")
                        .long_name("E-mini S&P 500")
                        .primary_exchange("CME")
                        .industry("Financial")
                        .category("Indices")
                        .subcategory("Broad Market Equity Index")
                        .time_zone_id("US/Central")
                        .encode_proto(),
                ),
                contract_data_end(9001),
            ],
            expected_request: "9|8|9000|0|ES|FUT|202406|0|||CME||USD|||0|||",
            expected_count: 1,
            validations: Box::new(|contracts| {
                assert_eq!(contracts[0].contract.symbol, Symbol::from("ES"));
                assert_eq!(contracts[0].contract.security_type, SecurityType::Future);
                assert_eq!(contracts[0].contract.exchange, Exchange::from("CME"));
                assert_eq!(contracts[0].contract.contract_id, 551318584);
                assert_eq!(contracts[0].contract.multiplier, "50");
                assert_eq!(contracts[0].contract.last_trade_date_or_contract_month, "20240621");
                assert_eq!(contracts[0].long_name, "E-mini S&P 500");
                assert_eq!(contracts[0].category, "Indices");
            }),
        },
        ContractDetailsTestCase {
            name: "bond contract details",
            contract: Contract {
                symbol: Symbol::from("TLT"),
                security_type: SecurityType::Bond,
                exchange: Exchange::from("SMART"),
                currency: Currency::from("USD"),
                ..Default::default()
            },
            ordered_responses: vec![
                proto_response(
                    IncomingMessages::ContractData,
                    contract_data()
                        .request_id(9001)
                        .contract_id(12345)
                        .symbol("TLT")
                        .security_type("BOND")
                        .last_trade_date_or_contract_month("20420815")
                        .exchange("SMART")
                        .currency("USD")
                        .local_symbol("TLT")
                        .trading_class("TLT")
                        .market_name("US Treasury Bond")
                        .order_types(STK_ORDER_TYPES)
                        .valid_exchanges("SMART,NYSE")
                        .long_name("US Treasury Bond")
                        .primary_exchange("SMART")
                        .industry("Government")
                        .time_zone_id("US/Eastern")
                        .encode_proto(),
                ),
                contract_data_end(9001),
            ],
            expected_request: "9|8|9000|0|TLT|BOND||0|||SMART||USD|||0|||",
            expected_count: 1,
            validations: Box::new(|contracts| {
                assert_eq!(contracts[0].contract.symbol, Symbol::from("TLT"));
                assert_eq!(contracts[0].contract.security_type, SecurityType::Bond);
                assert_eq!(contracts[0].contract.currency, Currency::from("USD"));
                assert_eq!(contracts[0].contract.contract_id, 12345);
                assert_eq!(contracts[0].long_name, "US Treasury Bond");
                assert_eq!(contracts[0].industry, "Government");
                assert_eq!(contracts[0].contract.last_trade_date_or_contract_month, "20420815");
                assert_eq!(contracts[0].contract.exchange, Exchange::from("SMART"));
                assert_eq!(contracts[0].market_name, "US Treasury Bond");
            }),
        },
        ContractDetailsTestCase {
            name: "stock contract details - multiple exchanges",
            contract: Contract {
                symbol: Symbol::from("AAPL"),
                security_type: SecurityType::Stock,
                exchange: Exchange::from("SMART"),
                currency: Currency::from("USD"),
                ..Default::default()
            },
            ordered_responses: vec![
                proto_response(
                    IncomingMessages::ContractData,
                    contract_data()
                        .request_id(9001)
                        .contract_id(265598)
                        .symbol("AAPL")
                        .security_type("STK")
                        .exchange("SMART")
                        .currency("USD")
                        .local_symbol("AAPL")
                        .trading_class("NMS")
                        .market_name("NASDAQ")
                        .order_types(STK_ORDER_TYPES)
                        .valid_exchanges(STK_VALID_EXCHANGES)
                        .long_name("APPLE INC")
                        .primary_exchange("NASDAQ")
                        .industry("Computers")
                        .category("Computers")
                        .subcategory("Computers-Electronic")
                        .time_zone_id("US/Eastern")
                        .stock_type("COMMON")
                        .encode_proto(),
                ),
                proto_response(
                    IncomingMessages::ContractData,
                    contract_data()
                        .request_id(9001)
                        .contract_id(265598)
                        .symbol("AAPL")
                        .security_type("STK")
                        .exchange("NYSE")
                        .currency("USD")
                        .local_symbol("AAPL")
                        .trading_class("NMS")
                        .market_name("NYSE")
                        .order_types(STK_ORDER_TYPES)
                        .valid_exchanges("NYSE")
                        .long_name("APPLE INC")
                        .primary_exchange("NYSE")
                        .industry("Computers")
                        .category("Computers")
                        .subcategory("Computers-Electronic")
                        .time_zone_id("US/Eastern")
                        .stock_type("COMMON")
                        .encode_proto(),
                ),
                contract_data_end(9001),
            ],
            expected_request: "9|8|9000|0|AAPL|STK||0|||SMART||USD|||0|||",
            expected_count: 2,
            validations: Box::new(|contracts| {
                assert_eq!(contracts.len(), 2);
                assert_eq!(contracts[0].contract.symbol, Symbol::from("AAPL"));
                assert_eq!(contracts[0].contract.security_type, SecurityType::Stock);
                assert_eq!(contracts[0].contract.exchange, Exchange::from("SMART"));
                assert_eq!(contracts[0].contract.primary_exchange, Exchange::from("NASDAQ"));
                assert_eq!(contracts[1].contract.symbol, Symbol::from("AAPL"));
                assert_eq!(contracts[1].contract.security_type, SecurityType::Stock);
                assert_eq!(contracts[1].contract.exchange, Exchange::from("NYSE"));
                assert_eq!(contracts[1].contract.primary_exchange, Exchange::from("NYSE"));
                assert_eq!(contracts[0].contract.contract_id, 265598);
                assert_eq!(contracts[1].contract.contract_id, 265598);
            }),
        },
        ContractDetailsTestCase {
            name: "TSLA contract details - multiple exchanges (sync_tests)",
            contract: Contract::stock("TSLA").build(),
            ordered_responses: vec![
                proto_response(
                    IncomingMessages::ContractData,
                    contract_data()
                        .request_id(9001)
                        .contract_id(76792991)
                        .symbol("TSLA")
                        .security_type("STK")
                        .exchange("SMART")
                        .currency("USD")
                        .local_symbol("TSLA")
                        .trading_class("NMS")
                        .market_name("NMS")
                        .order_types(STK_ORDER_TYPES)
                        .valid_exchanges(STK_VALID_EXCHANGES)
                        .long_name("TESLA INC")
                        .primary_exchange("NASDAQ")
                        .industry("Consumer, Cyclical")
                        .category("Auto Manufacturers")
                        .subcategory("Auto-Cars/Light Trucks")
                        .time_zone_id("US/Eastern")
                        .stock_type("COMMON")
                        .encode_proto(),
                ),
                proto_response(
                    IncomingMessages::ContractData,
                    contract_data()
                        .request_id(9001)
                        .contract_id(76792991)
                        .symbol("TSLA")
                        .security_type("STK")
                        .exchange("AMEX")
                        .currency("USD")
                        .local_symbol("TSLA")
                        .trading_class("NMS")
                        .market_name("NMS")
                        .order_types(AMEX_ORDER_TYPES)
                        .valid_exchanges(STK_VALID_EXCHANGES)
                        .long_name("TESLA INC")
                        .primary_exchange("NASDAQ")
                        .industry("Consumer, Cyclical")
                        .category("Auto Manufacturers")
                        .subcategory("Auto-Cars/Light Trucks")
                        .time_zone_id("US/Eastern")
                        .stock_type("COMMON")
                        .encode_proto(),
                ),
                contract_data_end(9001),
            ],
            expected_request: "9|8|9000|0|TSLA|STK||0|||SMART||USD|||0|||",
            expected_count: 2,
            validations: Box::new(|contracts| {
                assert_eq!(contracts.len(), 2);
                assert_eq!(contracts[0].contract.exchange, Exchange::from("SMART"));
                assert_eq!(contracts[1].contract.exchange, Exchange::from("AMEX"));
                assert_eq!(contracts[0].contract.symbol, Symbol::from("TSLA"));
                assert_eq!(contracts[0].contract.security_type, SecurityType::Stock);
                assert_eq!(contracts[0].contract.currency, Currency::from("USD"));
                assert_eq!(contracts[0].contract.contract_id, 76792991);
                assert_eq!(contracts[1].contract.contract_id, 76792991);
                assert_eq!(contracts[0].order_types.len(), 70);
                assert_eq!(contracts[0].order_types[0], "ACTIVETIM");
                assert_eq!(contracts[1].order_types.len(), 42);
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
            ordered_responses: vec![proto_response(
                IncomingMessages::SymbolSamples,
                symbol_samples()
                    .request_id(9000)
                    .entry(symbol_samples_entry(12345, "AAPL").primary_exchange("NASDAQ").description("APPLE INC"))
                    .encode_proto(),
            )],
            expected_request: "81|9000|AAPL|",
            expected_count: 1,
        },
        MatchingSymbolsTestCase {
            name: "multiple matches",
            pattern: "AA",
            ordered_responses: vec![proto_response(
                IncomingMessages::SymbolSamples,
                symbol_samples()
                    .request_id(9000)
                    .entry(
                        symbol_samples_entry(67890, "AA")
                            .primary_exchange("SMART")
                            .description("ALCOA CORP")
                            .derivative_security_types(vec!["OPT".to_string()]),
                    )
                    .entry(symbol_samples_entry(12346, "AAPL").primary_exchange("SMART").description("APPLE INC"))
                    .encode_proto(),
            )],
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
            ordered_responses: vec![proto_response(
                IncomingMessages::MarketRule,
                market_rule(26)
                    .increment(0.0, 0.01)
                    .increment(1.0, 0.01)
                    .increment(5.0, 0.01)
                    .increment(0.05, 0.05)
                    .encode_proto(),
            )],
            expected_request: "91|26|",
            expected_price_increments: 4,
        },
        MarketRuleTestCase {
            name: "complex market rule",
            market_rule_id: 635,
            ordered_responses: vec![proto_response(
                IncomingMessages::MarketRule,
                market_rule(635)
                    .increment(0.0, 0.0001)
                    .increment(0.01, 0.001)
                    .increment(10.0, 0.01)
                    .encode_proto(),
            )],
            expected_request: "91|635|",
            expected_price_increments: 3,
        },
        MarketRuleTestCase {
            name: "market rule with 6 increments",
            market_rule_id: 239,
            ordered_responses: vec![proto_response(
                IncomingMessages::MarketRule,
                market_rule(239)
                    .increment(0.0, 0.01)
                    .increment(0.5, 0.01)
                    .increment(1.0, 0.01)
                    .increment(3.0, 0.01)
                    .increment(10000000000.0, 0.05)
                    .increment(10000000000.0, 0.1)
                    .encode_proto(),
            )],
            expected_request: "91|239|",
            expected_price_increments: 6,
        },
    ]
}

/// Test cases for smart components
pub fn smart_components_test_cases() -> Vec<SmartComponentsTestCase> {
    vec![
        SmartComponentsTestCase {
            name: "empty",
            bbo_exchange: "a0",
            ordered_responses: vec![proto_response(IncomingMessages::SmartComponents, smart_components().encode_proto())],
            expected_count: 0,
            expected_first: None,
        },
        SmartComponentsTestCase {
            name: "single component",
            bbo_exchange: "a6",
            ordered_responses: vec![proto_response(
                IncomingMessages::SmartComponents,
                smart_components().component(1, "NASDAQ", "Q").encode_proto(),
            )],
            expected_count: 1,
            expected_first: Some((1, "NASDAQ", "Q")),
        },
        SmartComponentsTestCase {
            name: "multi component",
            bbo_exchange: "a9",
            ordered_responses: vec![proto_response(
                IncomingMessages::SmartComponents,
                smart_components()
                    .component(1, "NYSE", "N")
                    .component(2, "NASDAQ", "Q")
                    .component(3, "ARCA", "P")
                    .encode_proto(),
            )],
            expected_count: 3,
            expected_first: Some((1, "NYSE", "N")),
        },
    ]
}

/// Test cases for option calculations.
pub fn option_calculation_test_cases() -> Vec<OptionCalculationTestCase> {
    vec![
        OptionCalculationTestCase {
            name: "calculate option price",
            contract: Contract {
                symbol: Symbol::from("AAPL"),
                security_type: SecurityType::Option,
                last_trade_date_or_contract_month: "20231215".to_string(),
                strike: 150.0,
                right: Some(OptionRight::Call),
                exchange: Exchange::from("SMART"),
                currency: Currency::from("USD"),
                multiplier: "100".to_string(),
                ..Default::default()
            },
            volatility: Some(0.3),
            option_price: None,
            underlying_price: 145.0,
            ordered_responses: vec![proto_response(
                IncomingMessages::TickOptionComputation,
                tick_option_computation()
                    .request_id(9000)
                    .tick_type(13)
                    .implied_volatility(0.3)
                    .delta(0.42)
                    .option_price(7.85)
                    .present_value_dividend(-0.03)
                    .gamma(0.65)
                    .vega(-0.002)
                    .theta(0.98)
                    .underlying_price(6.87)
                    .encode_proto(),
            )],
            expected_request_prefix: "54|3|9000|0|AAPL|OPT|20231215|150|C|100|SMART||USD||0.3|145|",
            expected_price: 7.85,
            expected_delta: 0.42,
        },
        OptionCalculationTestCase {
            name: "calculate implied volatility",
            contract: Contract {
                symbol: Symbol::from("AAPL"),
                security_type: SecurityType::Option,
                last_trade_date_or_contract_month: "20231215".to_string(),
                strike: 150.0,
                right: Some(OptionRight::Call),
                exchange: Exchange::from("SMART"),
                currency: Currency::from("USD"),
                multiplier: "100".to_string(),
                ..Default::default()
            },
            volatility: None,
            option_price: Some(5.0),
            underlying_price: 145.0,
            ordered_responses: vec![proto_response(
                IncomingMessages::TickOptionComputation,
                tick_option_computation()
                    .request_id(9000)
                    .tick_type(13)
                    .implied_volatility(0.25)
                    .delta(0.32)
                    .option_price(5.0)
                    .present_value_dividend(-0.02)
                    .gamma(0.45)
                    .vega(-0.001)
                    .theta(0.25)
                    .underlying_price(4.55)
                    .encode_proto(),
            )],
            expected_request_prefix: "54|3|9000|0|AAPL|OPT|20231215|150|C|100|SMART||USD||5|145|",
            expected_price: 5.0,
            expected_delta: 0.32,
        },
    ]
}

/// Test cases for option chain
pub fn option_chain_test_cases() -> Vec<OptionChainTestCase> {
    vec![OptionChainTestCase {
        name: "stock option chain",
        symbol: "AAPL",
        exchange: "SMART",
        security_type: SecurityType::Stock,
        contract_id: 0,
        ordered_responses: vec![
            proto_response(
                IncomingMessages::SecurityDefinitionOptionParameter,
                option_chain()
                    .request_id(9000)
                    .exchange("SMART")
                    .underlying_contract_id(265598)
                    .trading_class("GOOG")
                    .multiplier("100")
                    .expirations(
                        [
                            "20230120", "20230217", "20230317", "20230421", "20230519", "20230616", "20230721", "20230818", "20231215", "20240119",
                            "20240621", "20250117",
                        ]
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    )
                    .strikes(vec![
                        50.0, 55.0, 60.0, 65.0, 70.0, 75.0, 80.0, 85.0, 90.0, 95.0, 100.0, 105.0, 110.0, 115.0, 120.0, 125.0, 130.0, 135.0, 140.0,
                        145.0, 150.0, 155.0, 160.0, 165.0, 170.0, 175.0, 180.0, 185.0, 190.0, 195.0, 200.0,
                    ])
                    .encode_proto(),
            ),
            text_response("76|9000|"),
        ],
        expected_request: "78|9000|AAPL|SMART|STK|0|",
        expected_count: 1,
    }]
}

/// Test cases for verify contract
pub fn verify_contract_test_cases() -> Vec<VerifyContractTestCase> {
    vec![
        VerifyContractTestCase {
            name: "valid contract",
            contract: Contract {
                symbol: Symbol::from("AAPL"),
                security_type: SecurityType::Stock,
                exchange: Exchange::from("SMART"),
                currency: Currency::from("USD"),
                ..Default::default()
            },
            server_version: server_versions::SIZE_RULES,
            should_error: false,
            error_contains: None,
        },
        VerifyContractTestCase {
            name: "contract with security_id - old server",
            contract: Contract {
                symbol: Symbol::from("AAPL"),
                security_type: SecurityType::Stock,
                exchange: Exchange::from("SMART"),
                currency: Currency::from("USD"),
                security_id: "US0378331005".to_string(),
                security_id_type: Some(SecurityIdType::Isin),
                ..Default::default()
            },
            server_version: server_versions::SEC_ID_TYPE - 1,
            should_error: true,
            error_contains: Some("security ID type"),
        },
        VerifyContractTestCase {
            name: "contract with trading class - old server",
            contract: Contract {
                symbol: Symbol::from("AAPL"),
                security_type: SecurityType::Stock,
                exchange: Exchange::from("SMART"),
                currency: Currency::from("USD"),
                trading_class: "AAPL".to_string(),
                ..Default::default()
            },
            server_version: server_versions::TRADING_CLASS - 1,
            should_error: true,
            error_contains: Some("trading class"),
        },
        VerifyContractTestCase {
            name: "contract with primary exchange - old server",
            contract: Contract {
                symbol: Symbol::from("AAPL"),
                security_type: SecurityType::Stock,
                exchange: Exchange::from("SMART"),
                currency: Currency::from("USD"),
                primary_exchange: Exchange::from("NASDAQ"),
                ..Default::default()
            },
            server_version: server_versions::LINKING - 1,
            should_error: true,
            error_contains: Some("linking"),
        },
        VerifyContractTestCase {
            name: "contract with issuer ID - old server",
            contract: Contract {
                symbol: Symbol::from("AAPL"),
                security_type: SecurityType::Bond,
                exchange: Exchange::from("SMART"),
                currency: Currency::from("USD"),
                issuer_id: "Q123".to_string(),
                ..Default::default()
            },
            server_version: server_versions::BOND_ISSUERID - 1,
            should_error: true,
            error_contains: Some("bond issuer ID"),
        },
    ]
}

/// Test cases for StreamDecoder implementations
pub fn stream_decoder_test_cases() -> Vec<StreamDecoderTestCase> {
    vec![
        StreamDecoderTestCase {
            name: "valid option computation",
            message: proto_response(
                IncomingMessages::TickOptionComputation,
                tick_option_computation()
                    .request_id(9000)
                    .tick_type(13)
                    .implied_volatility(0.3)
                    .delta(0.35)
                    .option_price(5.25)
                    .present_value_dividend(-0.025)
                    .gamma(0.55)
                    .vega(-0.0015)
                    .theta(0.3)
                    .underlying_price(4.75)
                    .encode_proto(),
            ),
            expected_result: StreamDecoderResult::OptionComputation { price: 5.25, delta: 0.35 },
        },
        StreamDecoderTestCase {
            name: "valid option chain",
            message: proto_response(
                IncomingMessages::SecurityDefinitionOptionParameter,
                option_chain()
                    .request_id(9000)
                    .exchange("SMART")
                    .underlying_contract_id(265598)
                    .trading_class("AAPL")
                    .multiplier("100")
                    .expirations(vec!["20230120".to_string(), "20230217".to_string()])
                    .strikes(vec![50.0, 55.0, 60.0])
                    .encode_proto(),
            ),
            expected_result: StreamDecoderResult::OptionChain {
                exchange: "SMART".to_string(),
                underlying_conid: 265598,
            },
        },
        StreamDecoderTestCase {
            name: "option chain end of stream",
            message: text_response("76|9000|"),
            expected_result: StreamDecoderResult::Error("EndOfStream"),
        },
        StreamDecoderTestCase {
            name: "unexpected message type",
            message: text_response("1|9000|unexpected|"),
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
            expected_msg_id: Ok(OutgoingMessages::CancelImpliedVolatility as i32 + 200),
        },
        CancelMessageTestCase {
            name: "cancel option price calculation",
            decoder_type: "OptionComputation",
            request_id: Some(9001),
            request_type: Some(OutgoingMessages::ReqCalcOptionPrice),
            expected_msg_id: Ok(OutgoingMessages::CancelOptionPrice as i32 + 200),
        },
        CancelMessageTestCase {
            name: "cancel option chain - not supported",
            decoder_type: "OptionChain",
            request_id: Some(9003),
            request_type: None,
            expected_msg_id: Err("cancel not implemented"),
        },
    ]
}

#[cfg(feature = "sync")]
/// Test case for client method tests (tests that use the Client convenience methods).
pub struct ClientMethodTestCase {
    pub name: &'static str,
    pub test_type: ClientMethodTest,
    pub ordered_responses: Vec<ResponseMessage>,
    #[allow(dead_code)]
    pub expected_request: &'static str,
    pub expected_result: ClientMethodResult,
}

#[cfg(feature = "sync")]
pub enum ClientMethodTest {
    CalculateOptionPrice {
        contract: Contract,
        volatility: f64,
        underlying_price: f64,
    },
    CalculateImpliedVolatility {
        contract: Contract,
        option_price: f64,
        underlying_price: f64,
    },
}

#[cfg(feature = "sync")]
pub enum ClientMethodResult {
    OptionComputation {
        option_price: Option<f64>,
        implied_volatility: Option<f64>,
    },
}

#[cfg(feature = "sync")]
/// Test cases for client method tests
pub fn client_method_test_cases() -> Vec<ClientMethodTestCase> {
    vec![
        ClientMethodTestCase {
            name: "calculate option price",
            test_type: ClientMethodTest::CalculateOptionPrice {
                contract: Contract::option("AAPL", "20231215", 150.0, OptionRight::Call),
                volatility: 0.25,
                underlying_price: 155.0,
            },
            ordered_responses: vec![proto_response(
                IncomingMessages::TickOptionComputation,
                tick_option_computation()
                    .request_id(9000)
                    .tick_type(13)
                    .implied_volatility(0.25)
                    .delta(0.42)
                    .option_price(85.5)
                    .present_value_dividend(-0.03)
                    .gamma(0.65)
                    .vega(-0.002)
                    .theta(0.98)
                    .underlying_price(6.87)
                    .encode_proto(),
            )],
            expected_request: "54|3|9000|0|AAPL|OPT|20231215|150|C||SMART||USD||0.25|155|",
            expected_result: ClientMethodResult::OptionComputation {
                option_price: Some(85.5),
                implied_volatility: Some(0.25),
            },
        },
        ClientMethodTestCase {
            name: "calculate implied volatility",
            test_type: ClientMethodTest::CalculateImpliedVolatility {
                contract: Contract::option("AAPL", "20231215", 150.0, OptionRight::Call),
                option_price: 8.5,
                underlying_price: 155.0,
            },
            ordered_responses: vec![proto_response(
                IncomingMessages::TickOptionComputation,
                tick_option_computation()
                    .request_id(9000)
                    .tick_type(13)
                    .implied_volatility(0.45)
                    .delta(0.32)
                    .option_price(8.5)
                    .present_value_dividend(-0.02)
                    .gamma(0.45)
                    .vega(-0.001)
                    .theta(0.25)
                    .underlying_price(4.55)
                    .encode_proto(),
            )],
            expected_request: "54|3|9000|0|AAPL|OPT|20231215|150|C||SMART||USD||8.5|155|",
            expected_result: ClientMethodResult::OptionComputation {
                implied_volatility: Some(0.45),
                option_price: Some(8.5),
            },
        },
    ]
}

#[cfg(feature = "sync")]
/// Test case for contract details error handling
pub struct ContractDetailsErrorTestCase {
    pub name: &'static str,
    pub contract: Contract,
    pub ordered_responses: Vec<ResponseMessage>,
    pub should_error: bool,
    pub error_contains: Option<&'static str>,
    pub expected_count: usize,
}

#[cfg(feature = "sync")]
/// Test cases for contract details error handling
pub fn contract_details_error_test_cases() -> Vec<ContractDetailsErrorTestCase> {
    vec![
        ContractDetailsErrorTestCase {
            name: "error message from server",
            contract: Contract::stock("INVALID").build(),
            ordered_responses: vec![proto_error_response(9000, 200, "Invalid contract")],
            should_error: true,
            error_contains: Some("Invalid contract"),
            expected_count: 0,
        },
        ContractDetailsErrorTestCase {
            name: "empty response (no contracts found)",
            contract: Contract::stock("NOEXIST").build(),
            ordered_responses: vec![contract_data_end(9000)],
            should_error: false,
            error_contains: None,
            expected_count: 0,
        },
        ContractDetailsErrorTestCase {
            name: "unexpected message type",
            contract: Contract::stock("AAPL").build(),
            ordered_responses: vec![text_response("79|9000|0|")],
            should_error: true,
            error_contains: Some("UnexpectedResponse"),
            expected_count: 0,
        },
        ContractDetailsErrorTestCase {
            name: "stream closed without ContractDataEnd",
            contract: Contract::stock("AAPL").build(),
            ordered_responses: vec![],
            should_error: true,
            error_contains: Some("UnexpectedEndOfStream"),
            expected_count: 0,
        },
    ]
}
