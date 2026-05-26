use super::*;
use crate::common::test_utils::helpers::{
    assert_request, assert_tws_error_message, proto_error_response, proto_response, request_message_count, text_response, TEST_REQ_ID_FIRST,
};
use crate::contracts::common::test_tables::*;
use crate::contracts::{Currency, Exchange, OptionRight, Symbol};
use crate::messages::IncomingMessages;
use crate::server_versions;
use crate::stubs::MessageBusStub;
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::testdata::builders::contracts::{
    calculate_implied_volatility_request, calculate_option_price_request, cancel_contract_data_request, contract_data, contract_data_request,
    market_rule_request, matching_symbols_request, option_chain_request,
};
use crate::testdata::builders::ResponseProtoEncoder;
use futures::StreamExt;
use std::sync::Arc;

#[tokio::test]
async fn test_contract_details() {
    for test_case in contract_details_test_cases() {
        let message_bus = Arc::new(MessageBusStub::with_ordered_responses(test_case.ordered_responses.clone()));

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        let result = client.contract_details(&test_case.contract).await;

        assert_eq!(request_message_count(&message_bus), 1);
        assert_request(
            &message_bus,
            0,
            &contract_data_request().request_id(TEST_REQ_ID_FIRST).contract(&test_case.contract),
        );

        assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
        let contracts = result.unwrap();
        assert_eq!(contracts.len(), test_case.expected_count, "Test '{}' count mismatch", test_case.name);

        (test_case.validations)(&contracts);
    }
}

#[tokio::test]
async fn test_matching_symbols() {
    for test_case in matching_symbols_test_cases() {
        let message_bus = Arc::new(MessageBusStub::with_ordered_responses(test_case.ordered_responses.clone()));

        let client = Client::stubbed(message_bus.clone(), server_versions::BOND_ISSUERID);
        let result = client.matching_symbols(test_case.pattern).await;

        assert_eq!(request_message_count(&message_bus), 1);
        assert_request(
            &message_bus,
            0,
            &matching_symbols_request().request_id(TEST_REQ_ID_FIRST).pattern(test_case.pattern),
        );

        assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), test_case.expected_count, "Test '{}' count mismatch", test_case.name);
    }
}

#[tokio::test]
async fn test_market_rule() {
    for test_case in market_rule_test_cases() {
        let message_bus = Arc::new(MessageBusStub::with_ordered_responses(test_case.ordered_responses.clone()));

        let client = Client::stubbed(message_bus.clone(), server_versions::MARKET_RULES);
        let result = client.market_rule(test_case.market_rule_id).await;

        assert_eq!(request_message_count(&message_bus), 1);
        assert_request(&message_bus, 0, &market_rule_request().market_rule_id(test_case.market_rule_id));

        assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
        let rule = result.unwrap();
        assert_eq!(
            rule.price_increments.len(),
            test_case.expected_price_increments,
            "Test '{}' price increments mismatch",
            test_case.name
        );
    }
}

#[tokio::test]
async fn test_option_calculations() {
    for test_case in option_calculation_test_cases() {
        let message_bus = Arc::new(MessageBusStub::with_ordered_responses(test_case.ordered_responses.clone()));

        let client = Client::stubbed(message_bus.clone(), server_versions::REQ_CALC_OPTION_PRICE);

        let result = if let Some(volatility) = test_case.volatility {
            let res = client
                .calculate_option_price(&test_case.contract, volatility, test_case.underlying_price)
                .await;
            assert_request(
                &message_bus,
                0,
                &calculate_option_price_request()
                    .request_id(TEST_REQ_ID_FIRST)
                    .contract(&test_case.contract)
                    .volatility(volatility)
                    .underlying_price(test_case.underlying_price),
            );
            res
        } else if let Some(option_price) = test_case.option_price {
            let res = client
                .calculate_implied_volatility(&test_case.contract, option_price, test_case.underlying_price)
                .await;
            assert_request(
                &message_bus,
                0,
                &calculate_implied_volatility_request()
                    .request_id(TEST_REQ_ID_FIRST)
                    .contract(&test_case.contract)
                    .option_price(option_price)
                    .underlying_price(test_case.underlying_price),
            );
            res
        } else {
            panic!("Test case must have either volatility or option_price");
        };

        assert_eq!(request_message_count(&message_bus), 1);

        assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
        let computation = result.unwrap();
        assert_eq!(
            computation.option_price,
            Some(test_case.expected_price),
            "Test '{}' price mismatch",
            test_case.name
        );
        assert_eq!(
            computation.delta,
            Some(test_case.expected_delta),
            "Test '{}' delta mismatch",
            test_case.name
        );
    }
}

#[tokio::test]
async fn test_option_chain() {
    for test_case in option_chain_test_cases() {
        let message_bus = Arc::new(MessageBusStub::with_ordered_responses(test_case.ordered_responses.clone()));

        let client = Client::stubbed(message_bus.clone(), server_versions::SEC_DEF_OPT_PARAMS_REQ);
        let result = client
            .option_chain(
                test_case.symbol,
                test_case.exchange,
                test_case.security_type.clone(),
                test_case.contract_id,
            )
            .await;

        assert_eq!(request_message_count(&message_bus), 1);
        assert_request(
            &message_bus,
            0,
            &option_chain_request()
                .request_id(TEST_REQ_ID_FIRST)
                .symbol(test_case.symbol)
                .exchange(test_case.exchange)
                .security_type(test_case.security_type.clone())
                .contract_id(test_case.contract_id),
        );

        assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
        let mut subscription = result.unwrap();

        let mut count = 0;
        while subscription.next().await.is_some() {
            count += 1;
        }
        assert_eq!(count, test_case.expected_count, "Test '{}' count mismatch", test_case.name);
    }
}

#[tokio::test]
async fn test_verify_contract() {
    for test_case in verify_contract_test_cases() {
        let message_bus = Arc::new(MessageBusStub::with_responses(vec![]));

        let client = Client::stubbed(message_bus, test_case.server_version);
        let result = verify::verify_contract(client.server_version(), &test_case.contract);

        if test_case.should_error {
            assert!(result.is_err(), "Test '{}' should have failed", test_case.name);
            if let Some(expected_error) = test_case.error_contains {
                let error_msg = format!("{:?}", result.err());
                assert!(
                    error_msg.contains(expected_error),
                    "Test '{}' error should contain '{}', got '{}'",
                    test_case.name,
                    expected_error,
                    error_msg
                );
            }
        } else {
            assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
        }
    }
}

#[tokio::test]
async fn test_stream_decoders() {
    for test_case in stream_decoder_test_cases() {
        let mut message = test_case.message.clone();

        match &test_case.expected_result {
            StreamDecoderResult::OptionComputation { price, delta } => {
                let result = OptionComputation::decode(&DecoderContext::new(server_versions::SIZE_RULES, None), &mut message);
                assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
                let computation = result.unwrap();
                assert_eq!(computation.option_price, Some(*price), "Test '{}' price mismatch", test_case.name);
                assert_eq!(computation.delta, Some(*delta), "Test '{}' delta mismatch", test_case.name);
            }
            StreamDecoderResult::OptionChain { exchange, underlying_conid } => {
                let result = OptionChain::decode(&DecoderContext::new(server_versions::SIZE_RULES, None), &mut message);
                assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
                let chain = result.unwrap();
                assert_eq!(chain.exchange, *exchange, "Test '{}' exchange mismatch", test_case.name);
                assert_eq!(
                    chain.underlying_contract_id, *underlying_conid,
                    "Test '{}' conid mismatch",
                    test_case.name
                );
            }
            StreamDecoderResult::Error(expected_error) => {
                if test_case.name == "option chain end of stream" {
                    let result = OptionChain::decode(&DecoderContext::new(server_versions::SIZE_RULES, None), &mut message);
                    assert!(result.is_err(), "Test '{}' should have failed", test_case.name);
                    assert!(
                        format!("{:?}", result.err()).contains(expected_error),
                        "Test '{}' wrong error",
                        test_case.name
                    );
                } else {
                    let opt_result = OptionComputation::decode(&DecoderContext::new(server_versions::SIZE_RULES, None), &mut message.clone());
                    let chain_result = OptionChain::decode(&DecoderContext::new(server_versions::SIZE_RULES, None), &mut message);
                    assert!(
                        opt_result.is_err() && chain_result.is_err(),
                        "Test '{}' should have failed",
                        test_case.name
                    );
                }
            }
        }
    }
}

#[tokio::test]
async fn test_cancel_messages() {
    for test_case in cancel_message_test_cases() {
        let context = test_case.request_type.map(|rt| DecoderContext::default().with_request_type(rt));

        let result = match test_case.decoder_type {
            "OptionComputation" => OptionComputation::cancel_message(server_versions::SIZE_RULES, test_case.request_id, context.as_ref()),
            "OptionChain" => {
                // OptionChain doesn't implement cancel_message
                Err(crate::Error::Simple("cancel not implemented".to_string()))
            }
            _ => panic!("Unknown decoder type"),
        };

        match &test_case.expected_msg_id {
            Ok(expected_id) => {
                assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
                let bytes = result.unwrap();
                let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                assert_eq!(msg_id, *expected_id, "Test '{}' message id mismatch", test_case.name);
            }
            Err(expected_error) => {
                assert!(result.is_err(), "Test '{}' should have failed", test_case.name);
                let error_msg = format!("{:?}", result.err());
                assert!(
                    error_msg.contains(expected_error),
                    "Test '{}' error should contain '{}', got '{}'",
                    test_case.name,
                    expected_error,
                    error_msg
                );
            }
        }
    }
}

#[tokio::test]
async fn request_stock_contract_details() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
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
                .long_name("TESLA INC")
                .primary_exchange("NASDAQ")
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
                .long_name("TESLA INC")
                .primary_exchange("NASDAQ")
                .stock_type("COMMON")
                .encode_proto(),
        ),
        text_response("52|1|9001|"),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    let contract = Contract::stock("TSLA").build();

    let results = client.contract_details(&contract).await;

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &contract_data_request().request_id(TEST_REQ_ID_FIRST).contract(&contract),
    );

    assert!(results.is_ok(), "failed to encode request: {:?}", results.err());

    let contracts: Vec<ContractDetails> = results.unwrap();
    assert_eq!(2, contracts.len());

    assert_eq!(contracts[0].contract.exchange, Exchange::from("SMART"));
    assert_eq!(contracts[1].contract.exchange, Exchange::from("AMEX"));

    assert_eq!(contracts[0].contract.symbol, Symbol::from("TSLA"));
    assert_eq!(contracts[0].contract.security_type, SecurityType::Stock);
    assert_eq!(contracts[0].contract.currency, Currency::from("USD"));
    assert_eq!(contracts[0].contract.contract_id, 76792991);
    assert_eq!(contracts[0].contract.primary_exchange, Exchange::from("NASDAQ"));
    assert_eq!(contracts[0].long_name, "TESLA INC");
    assert_eq!(contracts[0].stock_type, "COMMON");
    assert_eq!(contracts[0].min_size, 1.0);
    assert_eq!(contracts[0].size_increment, 1.0);
    assert_eq!(contracts[0].suggested_size_increment, 100.0);
}

#[tokio::test]
#[ignore = "reason: need sample messages"]
async fn request_bond_contract_details() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::ContractData,
            contract_data()
                .request_id(9001)
                .contract_id(12345)
                .symbol("TLT")
                .security_type("BOND")
                .last_trade_date_or_contract_month("20420815")
                .currency("USD")
                .local_symbol("TLT")
                .market_name("US Treasury Bond")
                .trading_class("BOND")
                .long_name("US Treasury Bond")
                .industry("Government")
                .encode_proto(),
        ),
        text_response("52|1|9001|"),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    // Create a bond contract
    let contract = Contract {
        symbol: Symbol::from("TLT"),
        security_type: SecurityType::Bond,
        exchange: Exchange::from("SMART"),
        currency: Currency::from("USD"),
        ..Default::default()
    };

    let results = client.contract_details(&contract).await;

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &contract_data_request().request_id(TEST_REQ_ID_FIRST).contract(&contract),
    );

    assert!(results.is_ok(), "failed to encode request: {:?}", results.err());

    let contracts: Vec<ContractDetails> = results.unwrap();
    assert_eq!(1, contracts.len());

    // Check basic contract fields
    assert_eq!(contracts[0].contract.symbol, Symbol::from("TLT"));
    assert_eq!(contracts[0].contract.security_type, SecurityType::Bond);
    assert_eq!(contracts[0].contract.currency, Currency::from("USD"));
    assert_eq!(contracts[0].contract.contract_id, 12345);

    // Check bond-specific fields
    assert_eq!(contracts[0].contract.last_trade_date_or_contract_month, "20420815");
    assert_eq!(contracts[0].cusip, "912810TL8");
    assert_eq!(contracts[0].coupon, 2.25);
    assert_eq!(contracts[0].maturity, "20420815");
    assert_eq!(contracts[0].issue_date, "20120815");
    assert_eq!(contracts[0].next_option_date, "20320815");
    assert_eq!(contracts[0].next_option_type, "CALL");
    assert_eq!(contracts[0].next_option_partial, true);
    assert_eq!(contracts[0].notes, "Government Bond Notes");
}

#[tokio::test]
async fn request_future_contract_details() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![
        proto_response(
            IncomingMessages::ContractData,
            contract_data()
                .request_id(9000)
                .contract_id(620731015)
                .symbol("ES")
                .security_type("FUT")
                .last_trade_date_or_contract_month("20250620")
                .multiplier("50")
                .exchange("CME")
                .currency("USD")
                .local_symbol("ESM5")
                .trading_class("ES")
                .market_name("ES")
                .min_tick("0.25")
                .long_name("E-mini S&P 500")
                .encode_proto(),
        ),
        text_response("52|1|9000|"),
    ]));

    let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);

    // Create a future contract
    let contract = Contract {
        symbol: Symbol::from("ES"),
        security_type: SecurityType::Future,
        exchange: Exchange::from("CME"),
        currency: Currency::from("USD"),
        last_trade_date_or_contract_month: "202506".to_string(),
        ..Default::default()
    };

    let results = client.contract_details(&contract).await;
    assert!(results.is_ok(), "failed to encode request: {:?}", results.err());

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(
        &message_bus,
        0,
        &contract_data_request().request_id(TEST_REQ_ID_FIRST).contract(&contract),
    );

    let contracts: Vec<ContractDetails> = results.unwrap();
    assert_eq!(1, contracts.len());

    assert_eq!(contracts[0].contract.symbol, Symbol::from("ES"));
    assert_eq!(contracts[0].contract.security_type, SecurityType::Future);
    assert_eq!(contracts[0].contract.exchange, Exchange::from("CME"));
    assert_eq!(contracts[0].contract.currency, Currency::from("USD"));
    assert_eq!(contracts[0].contract.contract_id, 620731015);
    assert_eq!(contracts[0].contract.local_symbol, "ESM5");
    assert_eq!(contracts[0].long_name, "E-mini S&P 500");
    assert_eq!(contracts[0].contract.multiplier, "50");
    assert_eq!(contracts[0].min_tick, 0.25);
}

#[tokio::test]
async fn test_cancel_contract_details() {
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![]));
    let client = Client::stubbed(message_bus.clone(), server_versions::CANCEL_CONTRACT_DATA);

    let result = client.cancel_contract_details(42).await;
    assert!(result.is_ok(), "cancel_contract_details failed: {:?}", result.err());

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &cancel_contract_data_request().request_id(42));
}

#[tokio::test]
async fn contract_details_returns_server_error() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_error_response(
        9000,
        200,
        "No security definition found",
    )]));
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let contract = Contract::stock("INVALID").build();

    let err = client.contract_details(&contract).await.unwrap_err();
    assert_tws_error_message(err, 200, "No security definition found");
}

#[tokio::test]
async fn contract_details_rejects_unexpected_message() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![text_response("79|9000|0|")]));
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let contract = Contract::stock("AAPL").build();

    let err = client.contract_details(&contract).await.unwrap_err();
    assert!(matches!(err, crate::Error::UnexpectedResponse(_)), "got {err:?}");
}

#[tokio::test]
async fn contract_details_returns_unexpected_end_of_stream() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![]));
    let client = Client::stubbed(message_bus, server_versions::SIZE_RULES);
    let contract = Contract::stock("AAPL").build();

    let err = client.contract_details(&contract).await.unwrap_err();
    assert!(matches!(err, crate::Error::UnexpectedEndOfStream), "got {err:?}");
}

#[tokio::test]
async fn contract_details_propagates_verify_failure() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![]));
    let client = Client::stubbed(message_bus.clone(), server_versions::TRADING_CLASS - 1);
    let contract = Contract::stock("AAPL").trading_class("NMS").build();

    let result = client.contract_details(&contract).await;
    assert!(matches!(result, Err(crate::Error::ServerVersion(..))), "got {result:?}");
    assert_eq!(request_message_count(&message_bus), 0);
}

#[tokio::test]
async fn matching_symbols_returns_server_error() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![text_response(
        "4|2|9000|321|invalid pattern|",
    )]));
    let client = Client::stubbed(message_bus, server_versions::BOND_ISSUERID);

    let err = client.matching_symbols("???").await.unwrap_err();
    assert!(matches!(err, crate::Error::UnexpectedResponse(_)), "got {err:?}");
}

#[tokio::test]
async fn matching_symbols_rejects_unexpected_message() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![text_response("10|9000|")]));
    let client = Client::stubbed(message_bus, server_versions::BOND_ISSUERID);

    let err = client.matching_symbols("AAPL").await.unwrap_err();
    assert!(matches!(err, crate::Error::UnexpectedResponse(_)), "got {err:?}");
}

#[tokio::test]
async fn matching_symbols_returns_empty_on_closed_stream() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![]));
    let client = Client::stubbed(message_bus, server_versions::BOND_ISSUERID);

    let symbols = client.matching_symbols("AAPL").await.expect("ok on empty stream");
    assert!(symbols.is_empty());
}

#[tokio::test]
async fn matching_symbols_rejects_old_server_version() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![]));
    let client = Client::stubbed(message_bus.clone(), server_versions::REQ_MATCHING_SYMBOLS - 1);

    let result = client.matching_symbols("AAPL").await;
    assert!(matches!(result, Err(crate::Error::ServerVersion(..))));
    assert_eq!(request_message_count(&message_bus), 0);
}

#[tokio::test]
async fn market_rule_returns_eof_on_empty_stream() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![]));
    let client = Client::stubbed(message_bus, server_versions::MARKET_RULES);

    let err = client.market_rule(26).await.unwrap_err();
    assert!(matches!(err, crate::Error::UnexpectedEndOfStream), "got {err:?}");
}

#[tokio::test]
async fn market_rule_rejects_old_server_version() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![]));
    let client = Client::stubbed(message_bus.clone(), server_versions::MARKET_RULES - 1);

    let result = client.market_rule(26).await;
    assert!(matches!(result, Err(crate::Error::ServerVersion(..))));
    assert_eq!(request_message_count(&message_bus), 0);
}

#[tokio::test]
async fn calculate_option_price_returns_eof_on_empty_stream() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![]));
    let client = Client::stubbed(message_bus, server_versions::REQ_CALC_OPTION_PRICE);
    let contract = Contract::option("AAPL", "20231215", 150.0, OptionRight::Call);

    let err = client.calculate_option_price(&contract, 0.25, 155.0).await.unwrap_err();
    assert!(matches!(err, crate::Error::UnexpectedEndOfStream), "got {err:?}");
}

#[tokio::test]
async fn calculate_option_price_rejects_old_server_version() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![]));
    let client = Client::stubbed(message_bus.clone(), server_versions::REQ_CALC_OPTION_PRICE - 1);
    let contract = Contract::option("AAPL", "20231215", 150.0, OptionRight::Call);

    let result = client.calculate_option_price(&contract, 0.25, 155.0).await;
    assert!(matches!(result, Err(crate::Error::ServerVersion(..))));
    assert_eq!(request_message_count(&message_bus), 0);
}

#[tokio::test]
async fn calculate_implied_volatility_returns_eof_on_empty_stream() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![]));
    let client = Client::stubbed(message_bus, server_versions::REQ_CALC_IMPLIED_VOLAT);
    let contract = Contract::option("AAPL", "20231215", 150.0, OptionRight::Call);

    let err = client.calculate_implied_volatility(&contract, 8.5, 155.0).await.unwrap_err();
    assert!(matches!(err, crate::Error::UnexpectedEndOfStream), "got {err:?}");
}

#[tokio::test]
async fn calculate_implied_volatility_rejects_old_server_version() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![]));
    let client = Client::stubbed(message_bus.clone(), server_versions::REQ_CALC_IMPLIED_VOLAT - 1);
    let contract = Contract::option("AAPL", "20231215", 150.0, OptionRight::Call);

    let result = client.calculate_implied_volatility(&contract, 8.5, 155.0).await;
    assert!(matches!(result, Err(crate::Error::ServerVersion(..))));
    assert_eq!(request_message_count(&message_bus), 0);
}

#[tokio::test]
async fn cancel_contract_details_rejects_old_server_version() {
    let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![]));
    let client = Client::stubbed(message_bus.clone(), server_versions::CANCEL_CONTRACT_DATA - 1);

    let result = client.cancel_contract_details(42).await;
    assert!(matches!(result, Err(crate::Error::ServerVersion(..))));
    assert_eq!(request_message_count(&message_bus), 0);
}
