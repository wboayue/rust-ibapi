use anyhow::{anyhow, Result};

use crate::client::Client;
use crate::domain::Contract;
use crate::domain::ContractDetails;

/// Requests contract information.
/// This method will provide all the contracts matching the contract provided. It can also be used to retrieve complete options and futures chains. This information will be returned at EWrapper:contractDetails. Though it is now (in API version > 9.72.12) advised to use reqSecDefOptParams for that purpose.
pub fn contract_details<C: Client>(client: &C, contract: &Contract) -> Result<ContractDetails> {
    Err(anyhow!("not implemented!"))
}
