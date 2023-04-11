use super::*;

pub(super) fn decode_head_timestamp(message: &mut ResponseMessage) -> Result<OffsetDateTime, Error> {
    message.skip(); // message type
    message.skip(); // request_id

    let head_timestamp = message.next_date_time()?;

    Ok(head_timestamp)
}

pub(super) fn decode_bar(message: &mut ResponseMessage) -> Result<Bar, Error> {
    message.skip(); // message type
    message.skip(); // request_id

    Ok(Bar {
        time: todo!(),
        open: todo!(),
        high: todo!(),
        low: todo!(),
        close: todo!(),
        volume: todo!(),
        wap: todo!(),
        count: todo!(),
    })
}

#[cfg(test)]
mod tests {
    use time::macros::{datetime, time};

    use super::*;

    #[test]
    fn decode_head_timestamp() {
        let mut message = ResponseMessage::from("88\09000\01560346200\0");

        let results = super::decode_head_timestamp(&mut message);

        if let Ok(head_timestamp) = results {
            assert_eq!(head_timestamp, datetime!(2019-06-12 13:30).assume_utc(), "head_timestamp");
        } else if let Err(err) = results {
            assert!(false, "error decoding trade tick: {err}");
        }
    }
}
