use crate::contracts::SecurityType;
use crate::messages::ResponseMessage;
use crate::{server_versions, Error};

use super::{AccountSummary, FamilyCode, PnL, PnLSingle, Position, PositionMulti};

pub(crate) fn decode_position(message: &mut ResponseMessage) -> Result<Position, Error> {
    message.skip(); // message type

    let message_version = message.next_int()?; // message version

    let mut position = Position {
        account: message.next_string()?,
        ..Default::default()
    };

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

    if message_version >= 2 {
        position.contract.trading_class = message.next_string()?;
    }

    position.position = message.next_double()?;

    if message_version >= 3 {
        position.average_cost = message.next_double()?;
    }

    Ok(position)
}

pub(crate) fn decode_position_multi(message: &mut ResponseMessage) -> Result<PositionMulti, Error> {
    message.skip(); // message type
    message.skip(); // message version
    message.skip(); // request id

    let mut position = PositionMulti {
        account: message.next_string()?,
        ..Default::default()
    };

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
    position.contract.trading_class = message.next_string()?;

    position.position = message.next_double()?;
    position.average_cost = message.next_double()?;
    position.model_code = message.next_string()?;

    Ok(position)
}

pub(crate) fn decode_family_codes(message: &mut ResponseMessage) -> Result<Vec<FamilyCode>, Error> {
    message.skip(); // message type

    let family_codes_count = message.next_int()?;

    if family_codes_count < 1 {
        return Ok(Vec::default());
    }

    let mut family_codes: Vec<FamilyCode> = Vec::with_capacity(family_codes_count as usize);

    for _ in 0..family_codes_count {
        let family_code = FamilyCode {
            account_id: message.next_string()?,
            family_code: message.next_string()?,
        };
        family_codes.push(family_code);
    }

    Ok(family_codes)
}

pub(crate) fn decode_pnl(server_version: i32, message: &mut ResponseMessage) -> Result<PnL, Error> {
    message.skip(); // message type
    message.skip(); // request id

    let daily_pnl = message.next_double()?;
    let unrealized_pnl = if server_version >= server_versions::UNREALIZED_PNL {
        Some(message.next_double()?)
    } else {
        None
    };
    let realized_pnl = if server_version >= server_versions::REALIZED_PNL {
        Some(message.next_double()?)
    } else {
        None
    };

    Ok(PnL {
        daily_pnl,
        unrealized_pnl,
        realized_pnl,
    })
}

pub(crate) fn decode_pnl_single(_server_version: i32, message: &mut ResponseMessage) -> Result<PnLSingle, Error> {
    message.skip(); // message type
    message.skip(); // request id

    let position = message.next_double()?;
    let daily_pnl = message.next_double()?;
    let unrealized_pnl = message.next_double()?;
    let realized_pnl = message.next_double()?;
    let value = message.next_double()?;

    Ok(PnLSingle {
        position,
        daily_pnl,
        unrealized_pnl,
        realized_pnl,
        value,
    })
}

pub(crate) fn decode_account_summary(_server_version: i32, message: &mut ResponseMessage) -> Result<AccountSummary, Error> {
    message.skip(); // message type
    message.skip(); // version
    message.skip(); // request id

    Ok(AccountSummary {
        account: message.next_string()?,
        tag: message.next_string()?,
        value: message.next_string()?,
        currency: message.next_string()?,
    })
}

#[cfg(test)]
mod tests;
