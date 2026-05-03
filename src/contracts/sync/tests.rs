use super::*;
use crate::common::test_utils::helpers::{assert_request, request_message_count, TEST_REQ_ID_FIRST};
use crate::contracts::common::test_tables::*;
use crate::messages::ResponseMessage;
use crate::server_versions;
use crate::stubs::MessageBusStub;
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::testdata::builders::contracts::{
    calculate_implied_volatility_request, calculate_option_price_request, cancel_contract_data_request, contract_data_request, market_rule_request,
    matching_symbols_request, option_chain_request,
};
use std::sync::{Arc, RwLock};

#[test]
fn test_contract_details() {
    for test_case in contract_details_test_cases() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: test_case.response_messages.clone(),
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        let result = client.contract_details(&test_case.contract);

        assert_eq!(request_message_count(&message_bus), 1, "Test '{}' request count", test_case.name);
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

#[test]
fn test_matching_symbols() {
    for test_case in matching_symbols_test_cases() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![test_case.response_message.clone()],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::BOND_ISSUERID);
        let result = client.matching_symbols(test_case.pattern);

        assert_eq!(request_message_count(&message_bus), 1, "Test '{}' request count", test_case.name);
        assert_request(
            &message_bus,
            0,
            &matching_symbols_request().request_id(TEST_REQ_ID_FIRST).pattern(test_case.pattern),
        );

        assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());
        let symbols: Vec<_> = result.unwrap().collect();
        assert_eq!(symbols.len(), test_case.expected_count, "Test '{}' count mismatch", test_case.name);
    }
}

#[test]
fn test_market_rule() {
    for test_case in market_rule_test_cases() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![test_case.response_message.clone()],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::MARKET_RULES);
        let result = client.market_rule(test_case.market_rule_id);

        assert_eq!(request_message_count(&message_bus), 1, "Test '{}' request count", test_case.name);
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

#[test]
fn test_option_calculations() {
    for test_case in option_calculation_test_cases() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![test_case.response_message.clone()],
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::REQ_CALC_OPTION_PRICE);

        let result = if let Some(volatility) = test_case.volatility {
            let res = client.calculate_option_price(&test_case.contract, volatility, test_case.underlying_price);
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
            let res = client.calculate_implied_volatility(&test_case.contract, option_price, test_case.underlying_price);
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

        assert_eq!(request_message_count(&message_bus), 1, "Test '{}' request count", test_case.name);

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

#[test]
fn test_option_chain() {
    for test_case in option_chain_test_cases() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: test_case.response_messages.clone(),
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SEC_DEF_OPT_PARAMS_REQ);
        let result = client.option_chain(
            test_case.symbol,
            test_case.exchange,
            test_case.security_type.clone(),
            test_case.contract_id,
        );

        assert_eq!(request_message_count(&message_bus), 1, "Test '{}' request count", test_case.name);
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
        let subscription = result.unwrap();

        let chains: Vec<_> = subscription.into_iter().collect();
        assert_eq!(chains.len(), test_case.expected_count, "Test '{}' count mismatch", test_case.name);
    }
}

#[test]
fn test_verify_contract() {
    for test_case in verify_contract_test_cases() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, test_case.server_version);
        let result = verify::verify_contract(client.server_version, &test_case.contract);

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

#[test]
fn test_stream_decoders() {
    for test_case in stream_decoder_test_cases() {
        let mut message = ResponseMessage::from(test_case.message);

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
                match test_case.message {
                    msg if msg.starts_with("76") => {
                        // OptionChain end of stream
                        let result = OptionChain::decode(&DecoderContext::new(server_versions::SIZE_RULES, None), &mut message);
                        assert!(result.is_err(), "Test '{}' should have failed", test_case.name);
                        assert!(
                            format!("{:?}", result.err()).contains(expected_error),
                            "Test '{}' wrong error",
                            test_case.name
                        );
                    }
                    _ => {
                        // Try both decoders
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
}

#[test]
fn test_cancel_messages() {
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

#[test]
fn test_client_methods() {
    for test_case in client_method_test_cases() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: test_case.response_messages.clone(),
        });

        let client = Client::stubbed(
            message_bus.clone(),
            match &test_case.test_type {
                ClientMethodTest::CalculateOptionPrice { .. } => server_versions::REQ_CALC_OPTION_PRICE,
                ClientMethodTest::CalculateImpliedVolatility { .. } => server_versions::REQ_CALC_IMPLIED_VOLAT,
            },
        );

        let result = match &test_case.test_type {
            ClientMethodTest::CalculateOptionPrice {
                contract,
                volatility,
                underlying_price,
            } => {
                let res = client.calculate_option_price(contract, *volatility, *underlying_price);
                assert_request(
                    &message_bus,
                    0,
                    &calculate_option_price_request()
                        .request_id(TEST_REQ_ID_FIRST)
                        .contract(contract)
                        .volatility(*volatility)
                        .underlying_price(*underlying_price),
                );
                res
            }
            ClientMethodTest::CalculateImpliedVolatility {
                contract,
                option_price,
                underlying_price,
            } => {
                let res = client.calculate_implied_volatility(contract, *option_price, *underlying_price);
                assert_request(
                    &message_bus,
                    0,
                    &calculate_implied_volatility_request()
                        .request_id(TEST_REQ_ID_FIRST)
                        .contract(contract)
                        .option_price(*option_price)
                        .underlying_price(*underlying_price),
                );
                res
            }
        };

        assert!(result.is_ok(), "Test '{}' failed: {:?}", test_case.name, result.err());

        assert_eq!(request_message_count(&message_bus), 1, "Test '{}' request count", test_case.name);

        let computation = result.unwrap();
        match &test_case.expected_result {
            ClientMethodResult::OptionComputation {
                option_price,
                implied_volatility,
            } => {
                assert_eq!(computation.option_price, *option_price, "Test '{}' option price mismatch", test_case.name);
                assert_eq!(
                    computation.implied_volatility, *implied_volatility,
                    "Test '{}' implied volatility mismatch",
                    test_case.name
                );
            }
        }
    }
}

#[test]
fn test_contract_details_errors() {
    for test_case in contract_details_error_test_cases() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: test_case.response_messages.clone(),
        });

        let client = Client::stubbed(message_bus.clone(), server_versions::SIZE_RULES);
        let result = client.contract_details(&test_case.contract);

        assert_eq!(request_message_count(&message_bus), 1, "Test '{}' request count", test_case.name);
        assert_request(
            &message_bus,
            0,
            &contract_data_request().request_id(TEST_REQ_ID_FIRST).contract(&test_case.contract),
        );

        if test_case.should_error {
            assert!(result.is_err(), "Test '{}' should have failed", test_case.name);
            if let Some(expected_error) = test_case.error_contains {
                let error_msg = result.unwrap_err().to_string();
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
            let contracts = result.unwrap();
            assert_eq!(contracts.len(), test_case.expected_count, "Test '{}' count mismatch", test_case.name);
        }
    }
}

#[test]
fn test_cancel_contract_details() {
    let message_bus = Arc::new(MessageBusStub::with_responses(vec![]));
    let client = Client::stubbed(message_bus.clone(), server_versions::CANCEL_CONTRACT_DATA);

    let result = client.cancel_contract_details(42);
    assert!(result.is_ok(), "cancel_contract_details failed: {:?}", result.err());

    assert_eq!(request_message_count(&message_bus), 1);
    assert_request(&message_bus, 0, &cancel_contract_data_request().request_id(42));
}
