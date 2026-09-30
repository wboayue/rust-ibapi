use super::*;
use crate::common::test_utils::helpers::proto_response;
use crate::messages::IncomingMessages;
use crate::testdata::builders::contracts::{contract_data, option_chain, symbol_samples, symbol_samples_entry};
use crate::testdata::builders::ResponseProtoEncoder;

#[test]
fn bounded_decode_limits_packed_and_unpacked_strikes_before_materialization() {
    let mut packed = option_chain().strikes(vec![1.0, 2.0, 3.0]).to_proto();
    packed.req_id = None;
    packed.underlying_con_id = None;
    packed.multiplier = None;
    // A packed field costs one field plus three values; unpacked values cost
    // one field apiece. Either representation must spend the same finite cap.
    let mut unpacked = Vec::new();
    for strike in [1.0_f64, 2.0, 3.0] {
        prost::encoding::encode_key(7, WireType::SixtyFourBit, &mut unpacked);
        unpacked.extend_from_slice(&strike.to_le_bytes());
    }
    for bytes in [packed.encode_to_vec(), unpacked] {
        let message = proto_response(IncomingMessages::SecurityDefinitionOptionParameter, bytes);
        let mut budget = Budget::new(QueryLimits {
            decode_entries: 2,
            ..Default::default()
        });
        assert!(matches!(
            chain(&message, &mut budget),
            Err(Error::ResponseLimitExceeded {
                resource: "decode entries",
                limit: 2
            })
        ));
        assert_eq!(
            chain(&message, &mut Budget::new(QueryLimits::default())).unwrap().strikes,
            [1.0, 2.0, 3.0]
        );
    }
}

#[test]
fn bounded_decode_budgets_nested_arrays_maps_and_repeated_message_merges() {
    let mut wire = contract_data().to_proto();
    let metadata = wire.contract_details.as_mut().unwrap();
    metadata.sec_id_list = (0..10).map(|i| (format!("scheme{i}"), format!("id{i}"))).collect();
    let message = proto_response(IncomingMessages::ContractData, wire.encode_to_vec());
    assert!(matches!(
        details(
            &message,
            &mut Budget::new(QueryLimits {
                decode_entries: 15,
                ..Default::default()
            })
        ),
        Err(Error::ResponseLimitExceeded {
            resource: "decode entries",
            ..
        })
    ));
    // Repeating a known optional message merges its fields in prost. Preflight
    // must count every occurrence, not just a final projected domain value.
    let mut bytes = Vec::new();
    for _ in 0..5 {
        prost::encoding::encode_key(2, WireType::LengthDelimited, &mut bytes);
        prost::encoding::encode_varint(2, &mut bytes);
        bytes.extend_from_slice(&[8, 1]); // Contract.con_id
    }
    let mut budget = Budget::new(QueryLimits {
        decode_entries: 9,
        ..Default::default()
    });
    assert!(matches!(
        scan(&bytes, Schema::Details, &mut budget),
        Err(Error::ResponseLimitExceeded { .. })
    ));
}

#[test]
fn bounded_decode_budgets_combo_legs_ineligibility_and_derivative_arrays() {
    for (schema, tag) in [(Schema::Contract, 20), (Schema::ContractMetadata, 58), (Schema::Description, 2)] {
        let mut bytes = Vec::new();
        for _ in 0..5 {
            prost::encoding::encode_key(tag, WireType::LengthDelimited, &mut bytes);
            prost::encoding::encode_varint(0, &mut bytes);
        }
        assert!(matches!(
            scan(
                &bytes,
                schema,
                &mut Budget::new(QueryLimits {
                    decode_entries: 4,
                    ..Default::default()
                })
            ),
            Err(Error::ResponseLimitExceeded { .. })
        ));
    }
}

#[test]
fn bounded_decode_row_limit_applies_to_native_samples_before_filtering() {
    let message = proto_response(
        IncomingMessages::SymbolSamples,
        symbol_samples()
            .entry(symbol_samples_entry(1, "ONE"))
            .entry(symbol_samples_entry(2, "TWO"))
            .encode_proto(),
    );
    assert!(matches!(
        symbols(
            &message,
            &mut Budget::new(QueryLimits {
                rows: 1,
                ..Default::default()
            })
        ),
        Err(Error::ResponseLimitExceeded { resource: "rows", limit: 1 })
    ));
    assert_eq!(symbols(&message, &mut Budget::new(QueryLimits::default())).unwrap().len(), 2);
}

#[test]
fn bounded_decode_limits_are_cumulative_across_frames() {
    let message = proto_response(IncomingMessages::ContractData, contract_data().encode_proto());
    let mut budget = Budget::new(QueryLimits {
        rows: 1,
        ..Default::default()
    });
    details(&message, &mut budget).unwrap();
    assert!(matches!(
        details(&message, &mut budget),
        Err(Error::ResponseLimitExceeded { resource: "rows", limit: 1 })
    ));
    let mut budget = Budget::new(QueryLimits::default());
    details(&message, &mut budget).unwrap();
    let first = budget.entries;
    budget.limits.decode_entries = first * 2 - 1;
    assert!(matches!(
        details(&message, &mut budget),
        Err(Error::ResponseLimitExceeded {
            resource: "decode entries",
            ..
        })
    ));
}

#[test]
fn bounded_preflight_rejects_malformed_lengths_nested_types_and_packed_values() {
    for (bytes, schema) in [
        (&[0x12, 0xff][..], Schema::Details),
        (&[0x10, 1], Schema::Details),
        (&[0x3a, 1, 0], Schema::Chain),
        (&[0x12, 10, 1], Schema::Symbols),
        (&[0], Schema::Chain),
    ] {
        assert!(scan(bytes, schema, &mut Budget::new(QueryLimits::default())).is_err());
    }
}

#[test]
fn bounded_preflight_runs_before_domain_string_decoding() {
    // Invalid UTF-8 in the fourth string. With a three-field budget the local
    // limit must win before prost attempts string allocation/validation.
    let bytes = [0x12, 0, 0x12, 0, 0x12, 0, 0x12, 1, 0xff];
    let message = proto_response(IncomingMessages::SecurityDefinitionOptionParameter, bytes.to_vec());
    assert!(matches!(
        chain(
            &message,
            &mut Budget::new(QueryLimits {
                decode_entries: 3,
                ..Default::default()
            })
        ),
        Err(Error::ResponseLimitExceeded {
            resource: "decode entries",
            limit: 3
        })
    ));
    assert!(chain(&message, &mut Budget::new(QueryLimits::default())).is_err());
}

#[test]
fn bounded_preflight_skips_unknown_fields_without_domain_allocations() {
    let mut wire = option_chain().strikes(vec![1.0]).encode_proto();
    prost::encoding::encode_key(100, WireType::LengthDelimited, &mut wire);
    prost::encoding::encode_varint(3, &mut wire);
    wire.extend_from_slice(&[0xff; 3]);
    let message = proto_response(IncomingMessages::SecurityDefinitionOptionParameter, wire);
    assert_eq!(chain(&message, &mut Budget::new(QueryLimits::default())).unwrap().strikes, [1.0]);
}

/// A protobuf group is refused rather than skipped: prost's `skip_field` walks
/// every field inside a group without consulting the budget, so an unknown
/// group could carry any amount of decode work past `decode_entries`. TWS
/// schemas are proto3, which cannot declare groups.
#[test]
fn bounded_preflight_refuses_protobuf_groups() {
    let valid = option_chain().to_proto().encode_to_vec();
    let mut counted = Budget::new(QueryLimits::default());
    chain(
        &proto_response(IncomingMessages::SecurityDefinitionOptionParameter, valid.clone()),
        &mut counted,
    )
    .unwrap();
    // Room for the valid frame plus a few entries, far fewer than the group's 100.
    let limits = QueryLimits {
        decode_entries: counted.entries + 10,
        ..Default::default()
    };

    let mut wire = valid;
    prost::encoding::encode_key(99, WireType::StartGroup, &mut wire);
    for _ in 0..100 {
        prost::encoding::encode_key(1, WireType::Varint, &mut wire);
        prost::encoding::encode_varint(1, &mut wire);
    }
    prost::encoding::encode_key(99, WireType::EndGroup, &mut wire);
    let message = proto_response(IncomingMessages::SecurityDefinitionOptionParameter, wire);

    let result = chain(&message, &mut Budget::new(limits));
    assert!(matches!(result, Err(Error::UnexpectedResponse(_))), "{result:?}");
}
