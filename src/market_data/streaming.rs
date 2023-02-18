use std::fmt::Debug;
use std::string::ToString;

use anyhow::{anyhow, Result};
use log::{error, info};

use crate::client::{Client, RequestMessage, ResponseMessage};
use crate::contracts::{Contract, ContractDetails};
use crate::messages::{IncomingMessages, OutgoingMessages};
use crate::orders::TagValue;
use crate::server_versions;

use super::{BarSize, WhatToShow};

pub fn realtime_bars<C: Client + Debug>(
    client: &mut C,
    contract: &Contract,
    bar_size: &BarSize,
    what_to_show: &WhatToShow,
    use_RTH: bool
) -> Result<Vec<ContractDetails>> {
    realtime_bars_with_options(client, contract, bar_size, what_to_show, use_RTH, Vec::default())
}

/// Requests contract information.
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
/// use ibapi::market_data::streaming;
///
/// fn main() -> anyhow::Result<()> {
///     let mut client = IBClient::connect("localhost:4002")?;
///
///     let contract = Contract::stock("TSLA");
///     let bars = streaming::realtime_bars(&mut client, &contract)?;
///
///     for bar in &bars {
///         println!("bar: {bar:?}");
///     }
///
///     Ok(())
/// }
/// ```
pub fn realtime_bars_with_options<C: Client + Debug>(
    client: &mut C,
    contract: &Contract,
    bar_size: &BarSize,
    what_to_show: &WhatToShow,
    use_RTH: bool,
    options: Vec<TagValue>
) -> Result<Vec<ContractDetails>> {
    client.check_server_version(
        server_versions::REAL_TIME_BARS,
        "It does not support real time bars.",
    )?;

    if !contract.trading_class.is_empty() || contract.contract_id > 0 {
        client.check_server_version(
            server_versions::TRADING_CLASS,
            "It does not support ConId nor TradingClass parameters in reqRealTimeBars.",
        )?;
    }

    const VERSION: i32 = 3;

    let request_id = client.next_request_id();
    let packet = encode_request_realtime_bars(client.server_version(), request_id, contract, bar_size, what_to_show, use_RTH, options)?;

    let responses = client.send_message_for_request(request_id, packet)?;

    let mut contract_details: Vec<ContractDetails> = Vec::default();


    Ok(Vec::default())
}

fn encode_request_realtime_bars(
    server_version: i32,
    ticker_id: i32,
    contract: &Contract,
    bar_size: &BarSize,
    what_to_show: &WhatToShow,
    use_RTH: bool,
    options: Vec<TagValue>
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

    packet.push_field(&0);      // bar size -- not used
    packet.push_field(&what_to_show.to_string());
    packet.push_field(&use_RTH);

    if server_version >= server_versions::LINKING {
        packet.push_field(&options);
    }

    Ok(packet)
}