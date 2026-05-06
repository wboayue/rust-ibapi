pub(super) mod decoders;
pub(crate) mod encoders;
/// Helpers for constructing commonly used order templates.
pub mod order_builder;
pub(crate) mod stream_decoders;
#[cfg(test)]
pub(super) mod test_data;
pub(super) mod verify;

// Narrow re-exports: only the handshake-time `_borrowed` adapters escape the
// `orders::common` boundary. The rest of `decoders` stays internal.
pub(crate) use decoders::{decode_open_order_borrowed, decode_order_status_borrowed};
