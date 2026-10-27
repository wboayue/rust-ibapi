pub(super) mod constants;
pub(super) mod decoders;
pub(super) mod encoders;
pub(super) mod snapshots;
pub(super) mod stream_decoders;

#[cfg(test)]
pub(super) mod test_data;

#[cfg(test)]
pub(super) mod test_tables;

// Narrow re-exports: only what the handshake path needs escapes the
// `accounts::common` boundary — the startup message dispatcher, and the
// account-list splitter `Client` uses for the handshake's managed accounts.
// The rest of `decoders` stays internal.
pub(crate) use decoders::{decode_account_update_message, split_account_list};
#[cfg(test)]
pub(crate) use encoders::encode_request_managed_accounts;
