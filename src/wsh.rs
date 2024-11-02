// Wall Street Horizon: Earnings Calendar & Event Data

use std::str;

use serde::{Deserialize, Serialize};

use crate::{
    client::{DataStream, ResponseContext, Subscription},
    messages::IncomingMessages,
    server_versions, Client, Error,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WshMetadata {
    data_json: String,
}

impl DataStream<WshMetadata> for WshMetadata {
    fn decode(_client: &Client, message: &mut crate::messages::ResponseMessage) -> Result<WshMetadata, Error> {
        match message.message_type() {
            IncomingMessages::WshMetaData => Ok(decoders::decode_wsh_metadata(message.clone())?),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: &ResponseContext) -> Result<crate::messages::RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel wsh metadata message.");
        encoders::encode_cancel_wsh_metadata(request_id)
    }
}

pub fn wsh_metadata(client: &Client) -> Result<Subscription<WshMetadata>, Error> {
    client.check_server_version(server_versions::WSHE_CALENDAR, "It does not support WSHE Calendar API.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_wsh_metadata(request_id)?;
    let subscription = client.send_request(request_id, request)?;
    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

mod encoders {
    use super::Error;

    use crate::messages::{OutgoingMessages, RequestMessage};

    pub(super) fn encode_request_wsh_metadata(request_id: i32) -> Result<RequestMessage, Error> {
        let mut message = RequestMessage::new();

        message.push_field(&OutgoingMessages::RequestWshMetaData);
        message.push_field(&request_id);

        Ok(message)
    }

    pub(super) fn encode_cancel_wsh_metadata(request_id: i32) -> Result<RequestMessage, Error> {
        let mut message = RequestMessage::new();

        message.push_field(&OutgoingMessages::CancelWshMetaData);
        message.push_field(&request_id);

        Ok(message)
    }
}

mod decoders {
    use crate::messages::ResponseMessage;

    use super::{Error, WshMetadata};

    pub(super) fn decode_wsh_metadata(mut message: ResponseMessage) -> Result<WshMetadata, Error> {
        message.skip(); // skip message type
        message.skip(); // skip request id

        Ok(WshMetadata {
            data_json: message.next_string()?,
        })
    }
}
