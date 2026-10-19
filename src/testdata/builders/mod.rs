//! Typed builders for response messages used in tests.
//!
//! Each per-domain submodule exposes builder structs (one per response message) with
//! `Default` impls sourced from `crate::common::test_utils::helpers::constants` and
//! fluent setters for individual fields. Response builders implement
//! [`ResponseProtoEncoder`]; request builders implement [`RequestEncoder`].
//!
//! Builders never touch production code; they are a test-only convenience layer.

use crate::messages::{encode_protobuf_message, OutgoingMessages};
use prost::Message;

/// Common encoder behavior for outgoing-request builders.
///
/// Implementors define [`Proto`](Self::Proto), [`MSG_ID`](Self::MSG_ID), and
/// [`to_proto`](Self::to_proto). The trait provides default `encode_proto`
/// (proto bytes only) and `encode_request` (4-byte msg_id header + proto).
pub(crate) trait RequestEncoder {
    type Proto: prost::Message + Default + PartialEq + std::fmt::Debug;
    const MSG_ID: OutgoingMessages;

    fn to_proto(&self) -> Self::Proto;

    fn encode_proto(&self) -> Vec<u8> {
        self.to_proto().encode_to_vec()
    }

    fn encode_request(&self) -> Vec<u8> {
        encode_protobuf_message(Self::MSG_ID as i32, &self.encode_proto())
    }
}

/// Per-builder protobuf encoder for response messages.
///
/// Counterpart to [`RequestEncoder`] for the response side. Implementors
/// declare their proto type and how to populate it from builder state; the
/// trait provides `encode_proto` for free.
pub(crate) trait ResponseProtoEncoder {
    type Proto: prost::Message;

    fn to_proto(&self) -> Self::Proto;

    fn encode_proto(&self) -> Vec<u8> {
        self.to_proto().encode_to_vec()
    }
}

/// Generates a typed builder for single-`req_id` protobuf request messages.
///
/// Covers both cancel messages and request messages whose proto body is just
/// a `req_id` (e.g. `WshMetaDataRequest`). Production-side counterpart is
/// `proto::encoders::encode_cancel_by_id!`, which is similarly reused for
/// non-cancel requests with the same shape. The generated type owns a single
/// `request_id: i32` and emits `proto::$proto_type { req_id: Some(self.request_id) }`.
macro_rules! single_req_id_request_builder {
    ($builder:ident, $proto_type:ident, $msg_id:expr) => {
        #[derive(Clone, Copy, Debug)]
        pub struct $builder {
            pub request_id: i32,
        }

        impl Default for $builder {
            fn default() -> Self {
                Self {
                    request_id: $crate::common::test_utils::helpers::constants::TEST_TICKER_ID,
                }
            }
        }

        impl $builder {
            pub fn request_id(mut self, v: i32) -> Self {
                self.request_id = v;
                self
            }
        }

        impl $crate::testdata::builders::RequestEncoder for $builder {
            type Proto = $crate::proto::$proto_type;
            const MSG_ID: $crate::messages::OutgoingMessages = $msg_id;

            fn to_proto(&self) -> Self::Proto {
                $crate::proto::$proto_type {
                    req_id: Some(self.request_id),
                }
            }
        }
    };
}

/// Generates a builder for `*End` responses carrying only `req_id`.
///
/// Used by sentinel-style `*End` responses (e.g. `AccountSummaryEnd`,
/// `AccountUpdateMultiEnd`, `PositionMultiEnd`). The generated type owns a
/// single `request_id: i32` and implements [`ResponseProtoEncoder`].
macro_rules! request_id_response_builder {
    ($builder:ident, $proto_type:ident) => {
        #[derive(Clone, Debug)]
        pub struct $builder {
            pub request_id: i32,
        }

        impl Default for $builder {
            fn default() -> Self {
                Self {
                    request_id: $crate::common::test_utils::helpers::constants::TEST_TICKER_ID,
                }
            }
        }

        impl $crate::testdata::builders::ResponseProtoEncoder for $builder {
            type Proto = $crate::proto::$proto_type;

            fn to_proto(&self) -> Self::Proto {
                $crate::proto::$proto_type {
                    req_id: Some(self.request_id),
                }
            }
        }
    };
}

/// Generates a typed builder for empty-body (no-field) protobuf request messages.
///
/// Mirrors `proto::encoders::encode_empty_proto!` on the production side. The
/// generated type is a unit struct.
macro_rules! empty_request_builder {
    ($builder:ident, $proto_type:ident, $msg_id:expr) => {
        #[derive(Clone, Copy, Debug, Default)]
        pub struct $builder;

        impl $crate::testdata::builders::RequestEncoder for $builder {
            type Proto = $crate::proto::$proto_type;
            const MSG_ID: $crate::messages::OutgoingMessages = $msg_id;

            fn to_proto(&self) -> Self::Proto {
                $crate::proto::$proto_type {}
            }
        }
    };
}

#[cfg_attr(not(all(feature = "sync", feature = "async")), allow(dead_code))] // a single-feature build misses the other half's tests
pub(crate) mod accounts;

pub(crate) mod config;

#[cfg_attr(not(all(feature = "sync", feature = "async")), allow(dead_code))] // a single-feature build misses the other half's tests
pub(crate) mod contracts;

pub(crate) mod display_groups;

#[cfg_attr(not(all(feature = "sync", feature = "async")), allow(dead_code))] // a single-feature build misses the other half's tests
pub(crate) mod market_data;

#[cfg_attr(not(all(feature = "sync", feature = "async")), allow(dead_code))] // a single-feature build misses the other half's tests
pub(crate) mod news;

#[cfg_attr(not(all(feature = "sync", feature = "async")), allow(dead_code))] // a single-feature build misses the other half's tests
pub(crate) mod orders;

pub(crate) mod positions;

pub(crate) mod scanner;

pub(crate) mod wsh;
