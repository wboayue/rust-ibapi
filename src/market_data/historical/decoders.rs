use super::*;

pub(super) fn decode_head_timestamp(message: &mut ResponseMessage) -> Result<OffsetDateTime, Error> {
    message.skip(); // message type

    let _request_id = message.next_int()?;
    let head_timestamp = message.next_date_time()?;

    Ok(head_timestamp)
}
