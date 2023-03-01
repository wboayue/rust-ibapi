use std::fmt::Debug;
use std::string::ToString;

use anyhow::Result;
use log::error;

use crate::client::transport::ResponsePacketPromise;
use crate::client::{Client, RequestMessage, ResponseMessage};
use crate::contracts::Contract;
use crate::messages::{IncomingMessages, OutgoingMessages};
use crate::orders::TagValue;
use crate::server_versions;

use super::{BarSize, RealTimeBar, WhatToShow};

/// Requests realtime bars.
///
/// This method will provide all the contracts matching the contract provided. It can also be used to retrieve complete options and futures chains. Though it is now (in API version > 9.72.12) advised to use reqSecDefOptParams for that purpose.
///
/// # Arguments
/// * `client` - [Client] with an active connection to gateway.
/// * `contract` - The [Contract] used as sample to query the available contracts. Typically, it will contain the [Contract]'s symbol, currency, security_type, and exchange.
///
/// # Examples
///
/// ```no_run
/// use ibapi::client::IBClient;
/// use ibapi::contracts::{self, Contract};
/// use ibapi::market_data::{streaming, BarSize, WhatToShow};
///
/// fn main() -> anyhow::Result<()> {
///     let mut client = IBClient::connect("localhost:4002")?;
///
///     let contract = Contract::stock("TSLA");
///     let bars = streaming::realtime_bars(&mut client, &contract, &BarSize::Secs5, &WhatToShow::Trades, false)?;
///
///     for (i, bar) in bars.enumerate() {
///         println!("bar[{i}]: {bar:?}");
///
///         if i > 60 {
///             break;
///         }
///     }
///
///
///     Ok(())
/// }
/// ```
pub fn realtime_bars<C: Client + Debug>(
    client: &mut C,
    contract: &Contract,
    bar_size: &BarSize,
    what_to_show: &WhatToShow,
    use_rth: bool,
) -> Result<RealTimeBarIterator> {
    realtime_bars_with_options(client, contract, bar_size, what_to_show, use_rth, Vec::default())
}

pub fn realtime_bars_with_options<C: Client + Debug>(
    client: &mut C,
    contract: &Contract,
    bar_size: &BarSize,
    what_to_show: &WhatToShow,
    use_rth: bool,
    options: Vec<TagValue>,
) -> Result<RealTimeBarIterator> {
    client.check_server_version(server_versions::REAL_TIME_BARS, "It does not support real time bars.")?;

    if !contract.trading_class.is_empty() || contract.contract_id > 0 {
        client.check_server_version(
            server_versions::TRADING_CLASS,
            "It does not support ConId nor TradingClass parameters in reqRealTimeBars.",
        )?;
    }

    let request_id = client.next_request_id();
    let packet = encode_request_realtime_bars(client.server_version(), request_id, contract, bar_size, what_to_show, use_rth, options)?;

    let responses = client.send_request(request_id, packet)?;

    Ok(RealTimeBarIterator::new(client.server_version(), request_id, responses))
}

pub struct RealTimeBarIterator {
    server_version: i32,
    request_id: i32,
    responses: ResponsePacketPromise,
}

impl RealTimeBarIterator {
    fn new(server_version: i32, request_id: i32, responses: ResponsePacketPromise) -> RealTimeBarIterator {
        RealTimeBarIterator {
            server_version,
            request_id,
            responses,
        }
    }

    /// Cancels request to stream realtime bars
    fn cancel_realtime_bars(&mut self) {}
}

impl Iterator for RealTimeBarIterator {
    type Item = RealTimeBar;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(mut message) = self.responses.next() {
            match message.message_type() {
                IncomingMessages::RealTimeBars => {
                    let decoded = decode_realtime_bar(self.server_version, &mut message);

                    if let Ok(bar) = decoded {
                        return Some(bar);
                    }

                    error!("unexpected message: {:?}", decoded.err());
                    None
                }
                _ => {
                    error!("unexpected message: {message:?}");
                    None
                }
            }
        } else {
            None
        }
    }
}

impl Drop for RealTimeBarIterator {
    fn drop(&mut self) {
        self.cancel_realtime_bars()
    }
}

fn encode_request_realtime_bars(
    server_version: i32,
    ticker_id: i32,
    contract: &Contract,
    bar_size: &BarSize,
    what_to_show: &WhatToShow,
    use_rth: bool,
    options: Vec<TagValue>,
) -> Result<RequestMessage> {
    const VERSION: i32 = 8;

    let mut packet = RequestMessage::default();

    packet.push_field(&OutgoingMessages::RequestRealTimeBars);
    packet.push_field(&VERSION);
    packet.push_field(&ticker_id);

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

    packet.push_field(&0); // bar size -- not used
    packet.push_field(&what_to_show.to_string());
    packet.push_field(&use_rth);

    if server_version >= server_versions::LINKING {
        packet.push_field(&options);
    }

    Ok(packet)
}

fn decode_realtime_bar(_server_version: i32, message: &mut ResponseMessage) -> Result<RealTimeBar> {
    message.skip(); // message type

    let _message_version = message.next_int()?;
    let _request_id = message.next_int()?;
    let date = message.next_long()?; // long, convert to date
    let open = message.next_double()?;
    let high = message.next_double()?;
    let low = message.next_double()?;
    let close = message.next_double()?;
    let volume = message.next_double()?;
    let wap = message.next_double()?;
    let count = message.next_int()?;

    Ok(RealTimeBar {
        date: date.to_string(),
        open,
        high,
        low,
        close,
        volume,
        wap,
        count,
    })
}
