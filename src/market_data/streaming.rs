use std::fmt::Debug;
use std::string::ToString;

use anyhow::{anyhow, Result};
use log::{error, info};

use crate::client::{Client, RequestMessage, ResponseMessage};
use crate::contracts::{Contract, ContractDetails};
use crate::messages::{IncomingMessages, OutgoingMessages};
use crate::server_versions;

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
pub fn realtime_bars<C: Client + Debug>(
    client: &mut C,
    contract: &Contract,
) -> Result<Vec<ContractDetails>> {
    // if !contract.security_id_type.is_empty() || !contract.security_id.is_empty() {
    //     client.check_server_version(
    //         server_versions::SEC_ID_TYPE,
    //         "It does not support security_id_type or security_id attributes",
    //     )?
    // }

    Ok(Vec::default())
}
