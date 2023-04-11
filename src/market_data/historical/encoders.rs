use crate::messages::OutgoingMessages;

use super::*;

const DATE_FORMAT: i32 = 2; // 1 for yyyyMMdd HH:mm:ss, 2 for system time format in seconds.

impl ToField for OffsetDateTime {
    fn to_field(&self) -> String {
        "30 days".into()
    }
}

impl ToField for Option<OffsetDateTime> {
    fn to_field(&self) -> String {
        match self {
            Some(date_time) => date_time.to_field(),
            None => "".into(),
        }
    }
}

// Encodes the head timestamp request
pub(super) fn encode_head_timestamp(request_id: i32, contract: &Contract, what_to_show: WhatToShow, use_rth: bool) -> Result<RequestMessage, Error> {
    let mut packet = RequestMessage::default();

    packet.push_field(&OutgoingMessages::RequestHeadTimestamp);
    packet.push_field(&request_id);
    contract.push_fields(&mut packet);
    packet.push_field(&use_rth);
    packet.push_field(&what_to_show);
    packet.push_field(&DATE_FORMAT);

    Ok(packet)
}

// Encodes the historical data request
#[allow(clippy::too_many_arguments)]
pub(super) fn encode_historical_data(
    server_version: i32,
    request_id: i32,
    contract: &Contract,
    end_date: Option<OffsetDateTime>,
    duration: Duration,
    bar_size: BarSize,
    what_to_show: Option<WhatToShow>,
    use_rth: bool,
    keep_up_to_data: bool,
) -> Result<RequestMessage, Error> {
    const VERSION: i32 = 6;

    let mut packet = RequestMessage::default();

    packet.push_field(&OutgoingMessages::RequestHistoricalData);

    if server_version < server_versions::SYNT_REALTIME_BARS {
        packet.push_field(&VERSION);
    }

    packet.push_field(&request_id);

    if server_version >= server_versions::TRADING_CLASS {
        packet.push_field(&contract.contract_id);
    }

    packet.push_field(&contract.symbol);
    packet.push_field(&contract.security_type);
    packet.push_field(&contract.last_trade_date_or_contract_month);
    packet.push_field(&contract.strike);
    packet.push_field(&contract.right);
    packet.push_field(&contract.multiplier);
    packet.push_field(&contract.exchange);
    packet.push_field(&contract.primary_exchange);
    packet.push_field(&contract.currency);
    packet.push_field(&contract.local_symbol);

    if server_version >= server_versions::TRADING_CLASS {
        packet.push_field(&contract.trading_class);
    }

    packet.push_field(&contract.include_expired);

    packet.push_field(&end_date);
    packet.push_field(&bar_size);

    packet.push_field(&duration);
    packet.push_field(&use_rth);
    packet.push_field(&what_to_show);

    packet.push_field(&DATE_FORMAT);

    if contract.is_bag() {
        packet.push_field(&contract.combo_legs.len());

        for combo_leg in &contract.combo_legs {
            packet.push_field(&combo_leg.contract_id);
            packet.push_field(&combo_leg.ratio);
            packet.push_field(&combo_leg.action);
            packet.push_field(&combo_leg.exchange);
        }
    }

    if server_version >= server_versions::SYNT_REALTIME_BARS {
        packet.push_field(&keep_up_to_data);
    }

    Ok(packet)
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use crate::messages::OutgoingMessages;
    use crate::ToField;

    use super::*;

    #[test]
    fn test_encode_head_timestamp() {
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
                assert_eq!(message[17], DATE_FORMAT.to_field(), "message.date_format");
            }
            Err(err) => {
                assert!(false, "error encoding head_timestamp request: {err}");
            }
        }
    }

    #[test]
    fn test_encode_historical_data() {
        let request_id = 9000;
        let contract = Contract::stock("MSFT");
        let end_date = Some(datetime!(2023-04-10 14:00 UTC));
        let duration = 30.days();
        let bar_size = BarSize::Day;
        let what_to_show: Option<WhatToShow> = None;
        let use_rth = false;
        let keep_up_to_date = true;

        let message = super::encode_historical_data(server_versions::SYNT_REALTIME_BARS, request_id, &contract, end_date, duration, bar_size, what_to_show, use_rth, keep_up_to_date);

        match message {
            Ok(message) => {
                assert_eq!(message[0], OutgoingMessages::RequestHistoricalData.to_field(), "message.type");
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
                assert_eq!(message[15], end_date.to_field(), "message.end_date");
                assert_eq!(message[16], bar_size.to_field(), "message.bar_size");
                assert_eq!(message[17], duration.to_field(), "message.duration");
                assert_eq!(message[18], use_rth.to_field(), "message.use_rth");
                assert_eq!(message[19], what_to_show.to_field(), "message.what_to_show");
                assert_eq!(message[20], DATE_FORMAT.to_field(), "message.date_format");

                let mut i: usize = 21;
                if contract.is_bag() {
                    assert_eq!(message[i], contract.combo_legs.len().to_field(), "message.combo_legs_count"); i += 1;

                    for combo_leg in &contract.combo_legs {
                        assert_eq!(message[i], combo_leg.contract_id.to_field(), "message.contract_id"); i += 1;
                        assert_eq!(message[i], combo_leg.ratio.to_field(), "message.ratio"); i += 1;
                        assert_eq!(message[i], combo_leg.action.to_field(), "message.action"); i += 1;
                        assert_eq!(message[i], combo_leg.exchange.to_field(), "message.exchange"); i += 1;
                    }
                }

                assert_eq!(message[i], keep_up_to_date.to_field(), "message.keep_up_to_date");
            }
            Err(err) => {
                assert!(false, "error encoding historical data request: {err}");
            }
        }
    }
}
