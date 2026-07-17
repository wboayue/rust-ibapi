//! Encoders for configuration request messages.

use crate::messages::OutgoingMessages;
use crate::Error;

pub(in crate::config) fn encode_request_config(request_id: i32) -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_cancel_by_id!(request_id, ConfigRequest, OutgoingMessages::ReqConfig)
}

// Encoder body assertions live in the sync/async tests via `assert_request<B>(builder)`.
