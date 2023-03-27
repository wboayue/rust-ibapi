use crate::client::ResponseMessage;
use crate::contracts::SecurityType;
use crate::{Error};

use super::Position;

pub(crate) fn position(_server_version: i32, message: &mut ResponseMessage) -> Result<Position, Error> {
    message.skip(); // message type

    let message_version = message.next_int()?; // message version

    let mut position = Position::default();

    position.account = message.next_string()?;

    position.contract.contract_id = message.next_int()?;
    position.contract.symbol = message.next_string()?;
    position.contract.security_type = SecurityType::from(&message.next_string()?);
    position.contract.last_trade_date_or_contract_month = message.next_string()?;
    position.contract.strike = message.next_double()?;
    position.contract.right = message.next_string()?;
    position.contract.multiplier = message.next_string()?;
    position.contract.exchange = message.next_string()?;
    position.contract.currency = message.next_string()?;
    position.contract.local_symbol = message.next_string()?;

    if message_version >= 2  {
        position.contract.trading_class = message.next_string()?;
    }

    position.position = message.next_double()?;

    if message_version >= 3 {
        position.average_cost = message.next_double()?;
    }

    Ok(position)
}
