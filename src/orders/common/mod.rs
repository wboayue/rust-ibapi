pub(super) mod decoders;
pub(super) mod encoders;
pub(crate) mod fill;
/// Helpers for constructing commonly used order templates.
pub mod order_builder;
pub(crate) mod stream_decoders;
pub(super) mod verify;

#[cfg(test)]
pub(super) mod test_tables;

// Narrow re-exports: only the handshake-time decoders escape the
// `orders::common` boundary. The rest of `decoders` stays internal.
pub(crate) use decoders::{decode_commission_report, decode_completed_order, decode_execution_data, decode_open_order, decode_order_status};
#[cfg(all(test, feature = "sync"))]
pub(crate) use encoders::encode_place_order;
#[cfg(test)]
pub(crate) use encoders::{encode_execution_filter, encode_order, encode_order_cancel};
