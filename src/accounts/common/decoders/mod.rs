use time::OffsetDateTime;

use prost::Message;

use crate::contracts::{Contract, Currency, Exchange, SecurityType, Symbol};
use crate::messages::ResponseMessage;
use crate::proto::decoders::parse_f64 as parse_str_f64;
use crate::{proto, server_versions, Error};

use super::super::{
    AccountMultiValue, AccountPortfolioValue, AccountSummary, AccountUpdate, AccountUpdateTime, AccountValue, FamilyCode, PnL, PnLSingle, Position,
    PositionMulti,
};
use crate::messages::IncomingMessages;

pub(crate) fn decode_position(message: &mut ResponseMessage) -> Result<Position, Error> {
    message.skip(); // message type

    let message_version = message.next_int()?; // message version

    let mut position = Position {
        account: message.next_string()?,
        ..Default::default()
    };

    position.contract.contract_id = message.next_int()?;
    position.contract.symbol = Symbol::from(message.next_string()?);
    position.contract.security_type = SecurityType::from(&message.next_string()?);
    position.contract.last_trade_date_or_contract_month = message.next_string()?;
    position.contract.strike = message.next_double()?;
    position.contract.right = message.next_string()?;
    position.contract.multiplier = message.next_string()?;
    position.contract.exchange = Exchange::from(message.next_string()?);
    position.contract.currency = Currency::from(message.next_string()?);
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
    position.contract.symbol = Symbol::from(message.next_string()?);
    position.contract.security_type = SecurityType::from(&message.next_string()?);
    position.contract.last_trade_date_or_contract_month = message.next_string()?;
    position.contract.strike = message.next_double()?;
    position.contract.right = message.next_string()?;
    position.contract.multiplier = message.next_string()?;
    position.contract.exchange = Exchange::from(message.next_string()?);
    position.contract.currency = Currency::from(message.next_string()?);
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
    message.decode_proto_or_text(decode_account_summary_proto, |msg| {
        msg.skip(); // message type
        msg.skip(); // version
        msg.skip(); // request id
        Ok(AccountSummary {
            account: msg.next_string()?,
            tag: msg.next_string()?,
            value: msg.next_string()?,
            currency: msg.next_string()?,
        })
    })
}

pub(crate) fn decode_account_value(message: &mut ResponseMessage) -> Result<AccountValue, Error> {
    message.skip(); // message type

    let message_version = message.next_int()?;

    let mut account_value = AccountValue {
        key: message.next_string()?,
        value: message.next_string()?,
        currency: message.next_string()?,
        ..Default::default()
    };

    if message_version >= 2 {
        account_value.account = Some(message.next_string()?);
    }

    Ok(account_value)
}

pub(crate) fn decode_account_portfolio_value(server_version: i32, message: &mut ResponseMessage) -> Result<AccountPortfolioValue, Error> {
    message.skip(); // message type

    let message_version = message.next_int()?;

    let mut contract = Contract::default();
    // For older message versions, primary_exchange should be empty, not default "SMART"
    if message_version < 7 {
        contract.primary_exchange = Exchange::from("");
    }
    if message_version >= 6 {
        contract.contract_id = message.next_int()?;
    }
    contract.symbol = Symbol::from(message.next_string()?);
    contract.security_type = SecurityType::from(&message.next_string()?);
    contract.last_trade_date_or_contract_month = message.next_string()?;
    contract.strike = message.next_double()?;
    contract.right = message.next_string()?;
    if message_version >= 7 {
        contract.multiplier = message.next_string()?;
        contract.primary_exchange = Exchange::from(message.next_string()?);
    }
    contract.currency = Currency::from(message.next_string()?);
    if message_version >= 2 {
        contract.local_symbol = message.next_string()?;
    }
    if message_version >= 8 {
        contract.trading_class = message.next_string()?;
    }

    let mut portfolio_value = AccountPortfolioValue {
        contract,
        ..Default::default()
    };

    portfolio_value.position = message.next_double()?;
    portfolio_value.market_price = message.next_double()?;
    portfolio_value.market_value = message.next_double()?;
    if message_version >= 3 {
        portfolio_value.average_cost = message.next_double()?;
        portfolio_value.unrealized_pnl = message.next_double()?;
        portfolio_value.realized_pnl = message.next_double()?;
    }
    if message_version >= 4 {
        portfolio_value.account = Some(message.next_string()?);
    }
    if message_version == 6 && server_version == 39 {
        portfolio_value.contract.primary_exchange = Exchange::from(message.next_string()?)
    }

    Ok(portfolio_value)
}

pub(crate) fn decode_account_update_time(message: &mut ResponseMessage) -> Result<AccountUpdateTime, Error> {
    message.skip(); // message type
    message.skip(); // version

    Ok(AccountUpdateTime {
        timestamp: message.next_string()?,
    })
}

pub(crate) fn decode_server_time(message: &mut ResponseMessage) -> Result<OffsetDateTime, Error> {
    message.decode_proto_or_text(
        |bytes| {
            let proto = proto::CurrentTime::decode(bytes).map_err(|e| Error::Simple(format!("failed to decode CurrentTime: {e}")))?;
            let timestamp = proto.current_time.unwrap_or(0);
            OffsetDateTime::from_unix_timestamp(timestamp).map_err(|e| Error::Simple(format!("Error parsing date: {e}")))
        },
        |msg| {
            msg.skip(); // message type
            msg.skip(); // message version
            let timestamp = msg.next_long()?;
            OffsetDateTime::from_unix_timestamp(timestamp).map_err(|e| Error::Simple(format!("Error parsing date: {e}")))
        },
    )
}

pub(crate) fn decode_server_time_millis(message: &mut ResponseMessage) -> Result<OffsetDateTime, Error> {
    message.decode_proto_or_text(
        |bytes| {
            let proto = proto::CurrentTimeInMillis::decode(bytes).map_err(|e| Error::Simple(format!("failed to decode CurrentTimeInMillis: {e}")))?;
            let millis = proto.current_time_in_millis.unwrap_or(0);
            OffsetDateTime::from_unix_timestamp_nanos(millis as i128 * 1_000_000).map_err(|e| Error::Simple(format!("Error parsing date: {e}")))
        },
        |msg| {
            msg.skip(); // message type
            let millis = msg.next_long()?;
            OffsetDateTime::from_unix_timestamp_nanos(millis as i128 * 1_000_000).map_err(|e| Error::Simple(format!("Error parsing date: {e}")))
        },
    )
}

pub(crate) fn decode_account_multi_value(message: &mut ResponseMessage) -> Result<AccountMultiValue, Error> {
    message.skip(); // message type
    message.skip(); // message version
    message.skip(); // request id

    let value = AccountMultiValue {
        account: message.next_string()?,
        model_code: message.next_string()?,
        key: message.next_string()?,
        value: message.next_string()?,
        currency: message.next_string()?,
    };

    Ok(value)
}

// === Protobuf decoders ===

#[allow(dead_code)]
pub(crate) fn decode_position_proto(bytes: &[u8]) -> Result<Position, Error> {
    let p = proto::Position::decode(bytes)?;
    let contract = p.contract.as_ref().map(proto::decoders::decode_contract).unwrap_or_default();
    Ok(Position {
        account: p.account.unwrap_or_default(),
        contract,
        position: parse_str_f64(&p.position),
        average_cost: p.avg_cost.unwrap_or_default(),
    })
}

#[allow(dead_code)]
pub(crate) fn decode_account_value_proto(bytes: &[u8]) -> Result<AccountValue, Error> {
    let p = proto::AccountValue::decode(bytes)?;
    Ok(AccountValue {
        key: p.key.unwrap_or_default(),
        value: p.value.unwrap_or_default(),
        currency: p.currency.unwrap_or_default(),
        account: p.account_name,
    })
}

#[allow(dead_code)]
pub(crate) fn decode_account_portfolio_value_proto(bytes: &[u8]) -> Result<AccountPortfolioValue, Error> {
    let p = proto::PortfolioValue::decode(bytes)?;
    let contract = p.contract.as_ref().map(proto::decoders::decode_contract).unwrap_or_default();
    Ok(AccountPortfolioValue {
        contract,
        position: parse_str_f64(&p.position),
        market_price: p.market_price.unwrap_or_default(),
        market_value: p.market_value.unwrap_or_default(),
        average_cost: p.average_cost.unwrap_or_default(),
        unrealized_pnl: p.unrealized_pnl.unwrap_or_default(),
        realized_pnl: p.realized_pnl.unwrap_or_default(),
        account: p.account_name,
    })
}

#[allow(dead_code)]
pub(crate) fn decode_pnl_proto(bytes: &[u8]) -> Result<PnL, Error> {
    let p = proto::PnL::decode(bytes)?;
    Ok(PnL {
        daily_pnl: p.daily_pn_l.unwrap_or_default(),
        unrealized_pnl: proto::decoders::optional_f64(p.unrealized_pn_l),
        realized_pnl: proto::decoders::optional_f64(p.realized_pn_l),
    })
}

#[allow(dead_code)]
pub(crate) fn decode_pnl_single_proto(bytes: &[u8]) -> Result<PnLSingle, Error> {
    let p = proto::PnLSingle::decode(bytes)?;
    Ok(PnLSingle {
        position: parse_str_f64(&p.position),
        daily_pnl: p.daily_pn_l.unwrap_or_default(),
        unrealized_pnl: p.unrealized_pn_l.unwrap_or_default(),
        realized_pnl: p.realized_pn_l.unwrap_or_default(),
        value: p.value.unwrap_or_default(),
    })
}

#[allow(dead_code)]
pub(crate) fn decode_account_summary_proto(bytes: &[u8]) -> Result<AccountSummary, Error> {
    let p = proto::AccountSummary::decode(bytes)?;
    Ok(AccountSummary {
        account: p.account.unwrap_or_default(),
        tag: p.tag.unwrap_or_default(),
        value: p.value.unwrap_or_default(),
        currency: p.currency.unwrap_or_default(),
    })
}

#[allow(dead_code)]
pub(crate) fn decode_position_multi_proto(bytes: &[u8]) -> Result<PositionMulti, Error> {
    let p = proto::PositionMulti::decode(bytes)?;
    let contract = p.contract.as_ref().map(proto::decoders::decode_contract).unwrap_or_default();
    Ok(PositionMulti {
        account: p.account.unwrap_or_default(),
        contract,
        position: parse_str_f64(&p.position),
        average_cost: p.avg_cost.unwrap_or_default(),
        model_code: p.model_code.unwrap_or_default(),
    })
}

#[allow(dead_code)]
pub(crate) fn decode_account_multi_value_proto(bytes: &[u8]) -> Result<AccountMultiValue, Error> {
    let p = proto::AccountUpdateMulti::decode(bytes)?;
    Ok(AccountMultiValue {
        account: p.account.unwrap_or_default(),
        model_code: p.model_code.unwrap_or_default(),
        key: p.key.unwrap_or_default(),
        value: p.value.unwrap_or_default(),
        currency: p.currency.unwrap_or_default(),
    })
}

/// Decode an account-update frame (`AccountValue` / `PortfolioValue` /
/// `AccountUpdateTime` / `AccountDownloadEnd`) using protobuf or text format
/// based on `message.is_protobuf` and dispatch to the right [`AccountUpdate`]
/// variant. Used by the connection layer's startup callback path.
pub(crate) fn decode_account_update_either(server_version: i32, message: &mut ResponseMessage) -> Result<AccountUpdate, Error> {
    match message.message_type() {
        IncomingMessages::AccountValue => message
            .decode_proto_or_text(decode_account_value_proto, decode_account_value)
            .map(AccountUpdate::AccountValue),
        IncomingMessages::PortfolioValue => message
            .decode_proto_or_text(decode_account_portfolio_value_proto, |m| {
                decode_account_portfolio_value(server_version, m)
            })
            .map(AccountUpdate::PortfolioValue),
        IncomingMessages::AccountUpdateTime => decode_account_update_time(message).map(AccountUpdate::UpdateTime),
        IncomingMessages::AccountDownloadEnd => Ok(AccountUpdate::End),
        other => Err(Error::Simple(format!("not an account-update message: {other:?}"))),
    }
}

#[cfg(test)]
mod tests;
