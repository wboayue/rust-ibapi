use crate::contracts::Contract;
use crate::{Client, IbApiError};

#[derive(Debug)]
pub struct Position {
    pub account: String,
    pub contract: Contract,
    pub position: f64,
    pub average_cost: f64,
}

pub(crate) fn positions(client: &Client) -> Result<impl Iterator<Item = Position>, IbApiError> {
    Ok(Vec::<Position>::new().into_iter())
}
