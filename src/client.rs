use anyhow::{anyhow, Result};

use crate::domain::BidAsk;
use crate::domain::{Contract, ContractDetails, RealTimeBar, TagValue};

#[derive(Debug)]
pub struct Client<'a> {
    host: &'a str,
    port: i32,
    client_id: i32,
}

pub fn connect(host: &str, port: i32, client_id: i32) -> anyhow::Result<Client> {
    println!("Connect, world!");
    Ok(Client {
        host,
        port,
        client_id,
    })
}

impl Client<'_> {
    /// Requests contract information.
    /// This method will provide all the contracts matching the contract provided. It can also be used to retrieve complete options and futures chains. This information will be returned at EWrapper:contractDetails. Though it is now (in API version > 9.72.12) advised to use reqSecDefOptParams for that purpose.
    pub fn contract_details(&self, contract: &Contract) -> Result<ContractDetails> {
        Err(anyhow!("not implemented!"))
    }
}
