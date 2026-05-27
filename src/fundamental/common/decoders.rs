use prost::Message;

use crate::fundamental::FundamentalData;
use crate::messages::{IncomingMessages, ResponseMessage};
use crate::proto;
use crate::Error;

pub(crate) fn decode_fundamental_data(message: &ResponseMessage) -> Result<FundamentalData, Error> {
    decode_fundamental_data_proto(message.require_proto()?)
}

pub(crate) fn decode_fundamental_data_proto(bytes: &[u8]) -> Result<FundamentalData, Error> {
    let p = proto::FundamentalsData::decode(bytes)?;
    Ok(FundamentalData {
        data: p.data.unwrap_or_default(),
    })
}

/// Dispatch on incoming message type and forward to the typed decoder. Routes
/// `Error` frames into `Error::Notice` and any other variant into
/// `Error::UnexpectedResponse`.
pub(in crate::fundamental) fn decode_fundamental_data_message(message: &ResponseMessage) -> Result<FundamentalData, Error> {
    match message.message_type() {
        IncomingMessages::FundamentalData => decode_fundamental_data(message),
        IncomingMessages::Error => Err(Error::from(message)),
        _ => Err(Error::unexpected_response(message)),
    }
}
