use crate::Error;
use crate::client::RequestMessage;
use crate::messages::OutgoingMessages;

pub(crate) fn request_positions() -> Result<RequestMessage, Error> {
    const VERSION: i32 = 1;

    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestPositions);
    message.push_field(&VERSION);

    Ok(message)
}

#[cfg(test)]
mod tests {
    use crate::ToField;

    use super::*;

    #[test]
    fn request_positions() {
        let results = super::request_positions();

        match results {
            Ok(message) => {
                assert_eq!(message[0], OutgoingMessages::RequestPositions.to_field(), "message.type");
                assert_eq!(message[1], "1", "message.version");
            }
            Err(err) => {
                assert!(false, "error encoding request positions: {err}");
            }
        }
    }
}
