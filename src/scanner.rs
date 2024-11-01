use crate::{client::DataStream, messages::OutgoingMessages, Client, Error};

// Requests an XML list of scanner parameters valid in TWS.
pub(super) fn scanner_parameters(client: &Client) -> Result<String, Error> {
    let request = encoders::encode_scanner_parameters()?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestScannerParameters, request)?;
    match subscription.next() {
        Some(Ok(message)) => decoders::decode_scanner_parameters(message),
        Some(Err(Error::ConnectionReset)) => scanner_parameters(client),
        Some(Err(e)) => Err(e),
        None => Err(Error::UnexpectedEndOfStream),
    }
}

pub struct ScannerSubscription {}

pub enum Scanner {
    Data(ScannerData),
    End,
}

pub struct ScannerData {}

// impl DataStream<ScannerData> for ScannerData {
// }

pub(super) fn scanner_subscription(client: &Client, subscription: ScannerSubscription, filter: &[&str]) -> Result<String, Error> {
    Err(Error::NotImplemented)
}

mod encoders {
    use crate::messages::OutgoingMessages;
    use crate::messages::RequestMessage;
    use crate::Error;

    pub(super) fn encode_scanner_parameters() -> Result<RequestMessage, Error> {
        const VERSION: i32 = 1;

        let mut message = RequestMessage::new();

        message.push_field(&OutgoingMessages::RequestScannerParameters);
        message.push_field(&VERSION);

        Ok(message)
    }
}

mod decoders {
    use crate::messages::ResponseMessage;
    use crate::Error;

    pub(super) fn decode_scanner_parameters(mut message: ResponseMessage) -> Result<String, Error> {
        message.skip(); // skip message type
        message.skip(); // skip message version

        Ok(message.next_string()?)
    }
}
