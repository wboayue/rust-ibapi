pub(super) mod constants;
pub(super) mod decoders;
pub(super) mod encoders;
pub(super) mod snapshots;
pub(super) mod stream_decoders;

#[cfg(test)]
pub(super) mod test_data;

#[cfg(test)]
pub(super) mod test_tables;

// Narrow re-export: only the handshake-time message dispatcher escapes the
// `accounts::common` boundary. The rest of `decoders` stays internal.
pub(crate) use decoders::decode_account_update_message;
#[cfg(test)]
pub(crate) use encoders::encode_request_managed_accounts;
