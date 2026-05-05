pub(super) mod constants;
pub(super) mod decoders;
pub(crate) mod encoders;
pub(super) mod stream_decoders;

#[cfg(test)]
pub(super) mod test_data;

#[cfg(test)]
pub(super) mod test_tables;

// Narrow re-export: only the handshake-time `_either` helper escapes the
// `accounts::common` boundary. The rest of `decoders` stays internal.
pub(crate) use decoders::decode_account_update_either;
