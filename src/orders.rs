use std::fmt::Debug;

use anyhow::{anyhow, Result};

use crate::client::{Client, RequestPacket, ResponsePacket};
use crate::contracts::{Contract};


// place_order
pub fn place_order<C: Client + Debug>(
    client: &mut C,
    order_id: i32,
    contract: &Contract,
    order: i32,
) -> Result<()> {
    Ok(())
}

// cancel_order
pub fn cancel_order<C: Client + Debug>(
    client: &mut C,
    order_id: i32,
) -> Result<()> {
    Ok(())
}

pub fn check_order_status<C: Client + Debug>() {
    
}