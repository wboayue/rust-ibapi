use super::tests::make_bus;
use super::*;
use crate::common::test_utils::helpers::{binary_proto, error_frame};
use crate::contracts::{Contract, QueryDisposition, QueryLimits, SecurityType};
use crate::messages::encode_raw_length;
use crate::server_versions;
use crate::subscriptions::SubscriptionItem;
use crate::testdata::builders::contracts::*;
use crate::testdata::builders::{RequestEncoder, ResponseProtoEncoder};

const WAIT: Duration = Duration::from_secs(5);

#[tokio::test]
async fn prepared_bounded_details_exposes_id_and_native_empty_end() {
    let (stream, bus) = make_bus();
    let client = crate::Client::stubbed(bus.clone(), server_versions::CANCEL_CONTRACT_DATA);
    let contract = Contract::stock("SYNTH").build();
    let mut query = client.prepare_contract_details(&contract, QueryLimits::default()).unwrap();
    let id = query.request_id();
    assert_eq!(query.disposition(), QueryDisposition::NotSubmitted);
    assert!(stream.captured().is_empty());
    assert!(matches!(query.next().await, Err(Error::InvalidArgument(_))));
    query.start().await.unwrap();
    assert_eq!(
        stream.captured(),
        encode_raw_length(&contract_data_request().request_id(id).contract(&contract).encode_request())
    );
    assert!(matches!(query.start().await, Err(Error::InvalidArgument(_))));
    assert_eq!(query.disposition(), QueryDisposition::Pending);
    stream.push_inbound(binary_proto(IncomingMessages::ContractDataEnd as i32, &contract_data_end(id).to_proto()));
    bus.read_and_route_message().await.unwrap();
    assert!(query.next().await.unwrap().is_none());
    assert_eq!(query.disposition(), QueryDisposition::ResponseEnded);
    assert!(!query.request_cancel().await.unwrap());
    drop(query);
    assert!(bus.is_connected());
    assert!(bus.bounded_requests.get(id).is_none());
}

#[tokio::test]
async fn bounded_details_preserves_prefix_notice_and_late_end_after_row_limit() {
    let (stream, bus) = make_bus();
    let client = crate::Client::stubbed(bus.clone(), server_versions::CANCEL_CONTRACT_DATA);
    let mut query = client
        .prepare_contract_details(
            &Contract::stock("SYNTH").build(),
            QueryLimits {
                rows: 1,
                ..Default::default()
            },
        )
        .unwrap();
    query.start().await.unwrap();
    let id = query.request_id();
    for frame in [
        error_frame(id, 2104, "synthetic advisory"),
        binary_proto(
            IncomingMessages::ContractData as i32,
            &contract_data().request_id(id).contract_id(11).to_proto(),
        ),
        binary_proto(
            IncomingMessages::ContractData as i32,
            &contract_data().request_id(id).contract_id(12).to_proto(),
        ),
    ] {
        stream.push_inbound(frame);
        bus.read_and_route_message().await.unwrap();
    }
    assert!(matches!(query.next().await.unwrap(), Some(SubscriptionItem::Notice(n)) if n.code == 2104));
    assert!(matches!(query.next().await.unwrap(), Some(SubscriptionItem::Data(d)) if d.contract.contract_id == 11));
    assert!(matches!(
        query.next().await,
        Err(Error::ResponseLimitExceeded { resource: "rows", limit: 1 })
    ));
    assert!(query.next().await.unwrap().is_none());
    assert_eq!(query.disposition(), QueryDisposition::Pending);
    assert!(query.request_cancel().await.unwrap());
    assert!(matches!(query.request_cancel().await, Err(Error::InvalidArgument(_))));
    assert!(bus.bounded_requests.get(id).is_some(), "cancel write must retain ownership");
    let mut cleanup = Box::pin(query.drain_until(tokio::time::Instant::now() + WAIT));
    assert!(futures::poll!(cleanup.as_mut()).is_pending(), "cancel write is not an end");
    stream.push_inbound(binary_proto(IncomingMessages::ContractDataEnd as i32, &contract_data_end(id).to_proto()));
    bus.read_and_route_message().await.unwrap();
    assert_eq!(cleanup.await.unwrap(), QueryDisposition::ResponseEnded);
    assert!(stream
        .captured()
        .ends_with(&encode_raw_length(&cancel_contract_data_request().request_id(id).encode_request())));
    drop(query);
    assert!(bus.is_connected());
}

#[tokio::test]
async fn bounded_cancel_version_gate_and_missing_end_retire() {
    for version in [server_versions::CANCEL_CONTRACT_DATA - 1, server_versions::CANCEL_CONTRACT_DATA] {
        let (stream, bus) = make_bus();
        let client = crate::Client::stubbed(bus.clone(), version);
        let mut query = client
            .prepare_contract_details(&Contract::stock("SYNTH").build(), QueryLimits::default())
            .unwrap();
        query.start().await.unwrap();
        let before = stream.captured();
        assert_eq!(query.request_cancel().await.unwrap(), version >= server_versions::CANCEL_CONTRACT_DATA);
        if version < server_versions::CANCEL_CONTRACT_DATA {
            assert_eq!(stream.captured(), before);
        }
        assert!(matches!(query.drain_until(tokio::time::Instant::now()).await, Err(Error::Io(ref e)) if e.kind() == std::io::ErrorKind::TimedOut));
        assert_eq!(query.disposition(), QueryDisposition::RetireRequired);
        assert!(!bus.is_connected());
    }
}

#[tokio::test]
async fn bounded_parameter_groups_have_no_cancel_or_stock_exchange_argument() {
    let (stream, bus) = make_bus();
    let client = crate::Client::stubbed(bus.clone(), server_versions::CANCEL_CONTRACT_DATA);
    let mut query = client
        .option_chain("SYNTH", SecurityType::Stock, 19)
        .prepare(QueryLimits::default())
        .unwrap();
    query.start().await.unwrap();
    let id = query.request_id();
    let bytes = stream.captured();
    let request: crate::proto::SecDefOptParamsRequest = prost::Message::decode(&bytes[8..]).unwrap();
    assert_eq!(request.req_id, Some(id));
    assert_eq!(request.underlying_con_id, Some(19));
    assert_eq!(request.fut_fop_exchange, None);
    assert!(!query.request_cancel().await.unwrap());
    assert_eq!(stream.captured(), bytes);
    stream.push_inbound(binary_proto(
        IncomingMessages::SecurityDefinitionOptionParameter as i32,
        &option_chain()
            .request_id(id)
            .exchange("SYNTH")
            .multiplier("10")
            .expirations(vec!["20261218".into()])
            .strikes(vec![1.0, 2.0])
            .to_proto(),
    ));
    bus.read_and_route_message().await.unwrap();
    assert!(matches!(query.next().await.unwrap(), Some(SubscriptionItem::Data(c)) if c.strikes == [1.0, 2.0] && c.multiplier == "10"));
    stream.push_inbound(binary_proto(
        IncomingMessages::SecurityDefinitionOptionParameterEnd as i32,
        &option_chain_end(id).to_proto(),
    ));
    bus.read_and_route_message().await.unwrap();
    assert_eq!(
        query.drain_until(tokio::time::Instant::now()).await.unwrap(),
        QueryDisposition::ResponseEnded
    );
    drop(query);
    assert!(bus.is_connected());
}

#[tokio::test]
async fn bounded_symbols_distinguish_empty_samples_from_transport_shutdown() {
    for response in [true, false] {
        let (stream, bus) = make_bus();
        let client = crate::Client::stubbed(bus.clone(), server_versions::PROTOBUF_REST_MESSAGES_3);
        let mut query = client.prepare_matching_symbols("SYNTH", QueryLimits::default()).unwrap();
        query.start().await.unwrap();
        let id = query.request_id();
        assert_eq!(
            stream.captured(),
            encode_raw_length(&matching_symbols_request().request_id(id).pattern("SYNTH").encode_request())
        );
        if response {
            stream.push_inbound(binary_proto(
                IncomingMessages::SymbolSamples as i32,
                &symbol_samples().request_id(id).to_proto(),
            ));
            bus.read_and_route_message().await.unwrap();
            assert!(matches!(query.next().await.unwrap(), Some(SubscriptionItem::Data(rows)) if rows.is_empty()));
            assert!(query.next().await.unwrap().is_none());
            assert_eq!(query.disposition(), QueryDisposition::ResponseEnded);
        } else {
            // Real dispatcher observes EOF and exhausts synthetic reconnects;
            // no live socket and no production backoff sleeping.
            stream.close();
            bus.clone().process_messages(0, Duration::ZERO).unwrap();
            // The dispatcher resets every registration before it tries to
            // reconnect, so the query ends with the reset, not an empty result.
            assert!(matches!(
                tokio::time::timeout(WAIT, query.next()).await.unwrap(),
                Err(Error::ConnectionReset)
            ));
            assert_eq!(query.disposition(), QueryDisposition::RetireRequired);
            bus.ensure_shutdown().await;
        }
    }
}

#[tokio::test]
async fn bounded_definition_rejection_is_not_an_empty_success_or_retirement() {
    let (stream, bus) = make_bus();
    let client = crate::Client::stubbed(bus.clone(), server_versions::CANCEL_CONTRACT_DATA);
    let mut query = client
        .prepare_contract_details(&Contract::stock("SYNTH").build(), QueryLimits::default())
        .unwrap();
    query.start().await.unwrap();
    stream.push_inbound(error_frame(query.request_id(), 200, "synthetic definition absent"));
    bus.read_and_route_message().await.unwrap();
    assert!(matches!(query.next().await, Err(Error::Notice(n)) if n.code == 200 && n.request_id == Some(query.request_id())));
    assert_eq!(query.disposition(), QueryDisposition::DefinitionRejected);
    assert_eq!(
        query.drain_until(tokio::time::Instant::now()).await.unwrap(),
        QueryDisposition::DefinitionRejected
    );
    drop(query);
    assert!(bus.is_connected());
}

#[tokio::test]
async fn bounded_query_drop_and_cleanup_future_drop_retire_only_after_submission() {
    for submitted in [false, true] {
        let (stream, bus) = make_bus();
        let client = crate::Client::stubbed(bus.clone(), server_versions::CANCEL_CONTRACT_DATA);
        let mut query = client
            .prepare_contract_details(&Contract::stock("SYNTH").build(), QueryLimits::default())
            .unwrap();
        let id = query.request_id();
        if submitted {
            query.start().await.unwrap();
        }
        drop(query);
        assert_eq!(bus.is_connected(), !submitted);
        assert!(bus.bounded_requests.get(id).is_none());
        assert_eq!(stream.captured().is_empty(), !submitted);
    }
    let (_, bus) = make_bus();
    let client = crate::Client::stubbed(bus.clone(), server_versions::CANCEL_CONTRACT_DATA);
    let mut query = client.prepare_matching_symbols("SYNTH", QueryLimits::default()).unwrap();
    query.start().await.unwrap();
    let mut cleanup = Box::pin(query.drain_until(tokio::time::Instant::now() + WAIT));
    assert!(futures::poll!(cleanup.as_mut()).is_pending());
    drop(cleanup);
    assert!(!bus.is_connected());
    assert_eq!(query.disposition(), QueryDisposition::RetireRequired);
}

#[tokio::test]
async fn bounded_reset_never_retries_and_stale_preparation_never_writes() {
    for start_before_reset in [false, true] {
        let (stream, bus) = make_bus();
        let client = crate::Client::stubbed(bus.clone(), server_versions::CANCEL_CONTRACT_DATA);
        let mut query = client.prepare_matching_symbols("SYNTH", QueryLimits::default()).unwrap();
        if start_before_reset {
            query.start().await.unwrap();
        }
        let bytes = stream.captured();
        bus.reset_channels().await;
        if start_before_reset {
            assert!(matches!(query.next().await, Err(Error::ConnectionReset)));
            assert_eq!(query.disposition(), QueryDisposition::RetireRequired);
        } else {
            assert!(matches!(query.start().await, Err(Error::ConnectionReset)));
            assert_eq!(query.disposition(), QueryDisposition::NotSubmitted);
        }
        assert_eq!(stream.captured(), bytes);
    }
}

#[tokio::test]
async fn bounded_invalid_limits_reject_before_writes() {
    let (stream, bus) = make_bus();
    let client = crate::Client::stubbed(bus.clone(), server_versions::CANCEL_CONTRACT_DATA);
    for limits in [
        QueryLimits {
            rows: 0,
            ..Default::default()
        },
        QueryLimits {
            frames: 0,
            ..Default::default()
        },
        QueryLimits {
            frame_bytes: 0,
            ..Default::default()
        },
        QueryLimits {
            total_bytes: 0,
            ..Default::default()
        },
        QueryLimits {
            decode_entries: 0,
            ..Default::default()
        },
    ] {
        assert!(matches!(client.prepare_matching_symbols("SYNTH", limits), Err(Error::InvalidArgument(_))));
    }
    assert!(stream.captured().is_empty());
}
