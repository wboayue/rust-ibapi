use super::*;

pub(super) fn decode_head_timestamp(message: &mut ResponseMessage) -> Result<OffsetDateTime, Error> {
    message.skip(); // message type
    message.skip(); // request_id

    let head_timestamp = message.next_date_time()?;

    Ok(head_timestamp)
}

pub(super) fn decode_historical_data(server_version: i32, message: &mut ResponseMessage) -> Result<HistoricalData, Error> {
    message.skip();     // message type

    let mut message_version = i32::MAX;
    if server_version < server_versions::SYNT_REALTIME_BARS {
        message_version = message.next_int()?;
    }

    message.skip();     // request_id

    let mut start_date = "".to_string();
    let mut end_date = "".to_string();
    if message_version > 2 {
        start_date = message.next_string()?;    
        end_date = message.next_string()?;    
    }

    let bars_count = message.next_int()?;
    
    let bars = Vec::new();

    Ok(HistoricalData {
        start_date,
        end_date,
        bars
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
