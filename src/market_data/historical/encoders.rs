use crate::messages::OutgoingMessages;

use super::*;

const HEAD_DATE_FORMAT: i32 = 2; // 1 for yyyyMMdd HH:mm:ss, 2 for system time format in seconds.

// Encodes the head timestamp request
pub(super) fn encode_head_timestamp(request_id: i32, contract: &Contract, what_to_show: WhatToShow, use_rth: bool) -> Result<RequestMessage, Error> {
    let mut packet = RequestMessage::default();

    packet.push_field(&OutgoingMessages::RequestHeadTimestamp);
    packet.push_field(&request_id);
    contract.push_fields(&mut packet);
    packet.push_field(&use_rth);
    packet.push_field(&what_to_show);
    packet.push_field(&HEAD_DATE_FORMAT);

    Ok(packet)
}

#[cfg(test)]
mod tests {
    use crate::messages::OutgoingMessages;
    use crate::ToField;

    use super::*;

    #[test]
    fn encode_head_timestamp() {
        let request_id = 9000;
        let contract = Contract::stock("MSFT");
        let what_to_show = WhatToShow::Trades;
        let use_rth = false;

        let results = super::encode_head_timestamp(request_id, &contract, what_to_show, use_rth);

        match results {
            Ok(message) => {
                assert_eq!(message[0], OutgoingMessages::RequestHeadTimestamp.to_field(), "message.type");
                assert_eq!(message[1], request_id.to_field(), "message.request_id");
                assert_eq!(message[2], contract.contract_id.to_field(), "message.contract_id");
                assert_eq!(message[3], contract.symbol, "message.symbol");
                assert_eq!(message[4], contract.security_type.to_field(), "message.security_type");
                assert_eq!(
                    message[5], contract.last_trade_date_or_contract_month,
                    "message.last_trade_date_or_contract_month"
                );
                assert_eq!(message[6], contract.strike.to_field(), "message.strike");
                assert_eq!(message[7], contract.right, "message.right");
                assert_eq!(message[8], contract.multiplier, "message.multiplier");
                assert_eq!(message[9], contract.exchange, "message.exchange");
                assert_eq!(message[10], contract.primary_exchange, "message.primary_exchange");
                assert_eq!(message[11], contract.currency, "message.currency");
                assert_eq!(message[12], contract.local_symbol, "message.local_symbol");
                assert_eq!(message[13], contract.trading_class, "message.trading_class");
                assert_eq!(message[14], contract.include_expired.to_field(), "message.include_expired");
                assert_eq!(message[15], use_rth.to_field(), "message.use_rth");
                assert_eq!(message[16], what_to_show.to_field(), "message.what_to_show");
                assert_eq!(message[17], HEAD_DATE_FORMAT.to_field(), "message.date_format");
            }
            Err(err) => {
                assert!(false, "error encoding head_timestamp request: {err}");
            }
        }
    }
}
