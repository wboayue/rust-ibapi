//! Typed builders for response messages used in tests.
//!
//! Each per-domain submodule exposes builder structs (one per response message) with
//! `Default` impls sourced from `crate::common::test_utils::helpers::constants` and
//! fluent setters for individual fields. Builders implement [`ResponseEncoder`],
//! which provides three on-the-wire formats:
//!
//! - `encode_pipe()` — pipe-delimited string for current `MessageBusStub` consumers.
//! - `encode_null()` — NUL-delimited string (post-conversion form).
//! - `encode_length_prefixed()` — 4-byte length prefix + NUL-delimited payload, for
//!   `MemoryStream`-style listener fixtures.
//!
//! Builders never touch production code; they are a test-only convenience layer.

use crate::messages::{encode_protobuf_message, encode_raw_length, OutgoingMessages};
use prost::Message;

/// Common encoder behavior for response builders.
///
/// Implementors only define [`fields`](Self::fields); the three encoder methods
/// derive their behavior from the field list, eliminating per-struct boilerplate.
pub(crate) trait ResponseEncoder {
    fn fields(&self) -> Vec<String>;

    fn encode_pipe(&self) -> String {
        join_fields(&self.fields(), '|')
    }

    fn encode_null(&self) -> String {
        join_fields(&self.fields(), '\0')
    }

    fn encode_length_prefixed(&self) -> Vec<u8> {
        encode_raw_length(self.encode_null().as_bytes())
    }
}

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

/// Generates a typed builder for cancel-by-request-id protobuf messages.
///
/// Mirrors `proto::encoders::encode_cancel_by_id!` on the production side.
/// The generated type owns a single `request_id: i32` and emits
/// `proto::$proto_type { req_id: Some(self.request_id) }`.
macro_rules! cancel_by_request_id_builder {
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

/// Generates a typed builder for response messages whose only payload is a
/// `request_id`: header (`msg_tag|"1"|request_id|`) + proto `req_id` field.
///
/// Used by sentinel-style `*End` responses (e.g. `AccountSummaryEnd`,
/// `AccountUpdateMultiEnd`, `PositionMultiEnd`). The generated type owns a
/// single `request_id: i32`, implements [`ResponseEncoder`] (for the text
/// path), and [`ResponseProtoEncoder`] (for protobuf round-trips).
macro_rules! request_id_response_builder {
    ($builder:ident, $msg_tag:literal, $proto_type:ident) => {
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

        impl $builder {
            pub fn request_id(mut self, v: i32) -> Self {
                self.request_id = v;
                self
            }
        }

        impl $crate::testdata::builders::ResponseEncoder for $builder {
            fn fields(&self) -> Vec<String> {
                vec![$msg_tag.to_string(), "1".to_string(), self.request_id.to_string()]
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

fn join_fields(fields: &[String], sep: char) -> String {
    let mut out = fields.join(&sep.to_string());
    out.push(sep);
    out
}

/// Collect `encode_pipe()` outputs from a heterogeneous list of builders, ready
/// to feed into `MessageBusStub::response_messages`.
///
/// Heterogeneous builders coerce to `&dyn ResponseEncoder` inside the slice
/// literal, so call sites stay terse:
/// ```ignore
/// let responses = response_messages(&[&position(), &position_end()]);
/// let (client, bus) = create_test_client_with_responses(responses);
/// ```
#[allow(dead_code)] // consumed by future domain test migrations
pub(crate) fn response_messages(builders: &[&dyn ResponseEncoder]) -> Vec<String> {
    builders.iter().map(|b| b.encode_pipe()).collect()
}

#[allow(dead_code)] // setters/encoders are consumed by future domain test migrations
pub(crate) mod accounts;

#[allow(dead_code)] // setters/encoders are consumed by future domain test migrations
pub(crate) mod positions;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
