use time::OffsetDateTime;

use prost::Message;

use crate::messages::ResponseMessage;
use crate::proto::decoders::parse_f64 as parse_str_f64;
use crate::{proto, Error};

use super::super::{
    AccountMultiValue, AccountPortfolioValue, AccountSummary, AccountUpdate, AccountUpdateTime, AccountValue, FaConfig, FaDataType, FamilyCode, PnL,
    PnLSingle, Position, PositionMulti, ReplaceFaResult, UserInfo, VerificationChallenge, VerificationResult,
};
use crate::messages::IncomingMessages;
use crate::orders::SoftDollarTier;

pub(crate) fn decode_position(message: &ResponseMessage) -> Result<Position, Error> {
    decode_position_proto(message.require_proto()?)
}

pub(crate) fn decode_position_multi(message: &ResponseMessage) -> Result<PositionMulti, Error> {
    decode_position_multi_proto(message.require_proto()?)
}

pub(crate) fn decode_family_codes(message: &ResponseMessage) -> Result<Vec<FamilyCode>, Error> {
    decode_family_codes_proto(message.require_proto()?)
}

pub(crate) fn decode_pnl(message: &ResponseMessage) -> Result<PnL, Error> {
    decode_pnl_proto(message.require_proto()?)
}

pub(crate) fn decode_pnl_single(message: &ResponseMessage) -> Result<PnLSingle, Error> {
    decode_pnl_single_proto(message.require_proto()?)
}

pub(crate) fn decode_account_summary(message: &ResponseMessage) -> Result<AccountSummary, Error> {
    decode_account_summary_proto(message.require_proto()?)
}

pub(crate) fn decode_account_value(message: &ResponseMessage) -> Result<AccountValue, Error> {
    decode_account_value_proto(message.require_proto()?)
}

pub(crate) fn decode_account_portfolio_value(message: &ResponseMessage) -> Result<AccountPortfolioValue, Error> {
    decode_account_portfolio_value_proto(message.require_proto()?)
}

pub(crate) fn decode_account_update_time(message: &ResponseMessage) -> Result<AccountUpdateTime, Error> {
    decode_account_update_time_proto(message.require_proto()?)
}

pub(crate) fn decode_server_time(message: &ResponseMessage) -> Result<OffsetDateTime, Error> {
    decode_server_time_proto(message.require_proto()?)
}

pub(crate) fn decode_server_time_millis(message: &ResponseMessage) -> Result<OffsetDateTime, Error> {
    decode_server_time_millis_proto(message.require_proto()?)
}

pub(crate) fn decode_server_time_proto(bytes: &[u8]) -> Result<OffsetDateTime, Error> {
    let proto = proto::CurrentTime::decode(bytes)?;
    let timestamp = proto.current_time.unwrap_or(0);
    OffsetDateTime::from_unix_timestamp(timestamp).map_err(|e| Error::parse_proto("current_time", e.to_string()))
}

pub(crate) fn decode_server_time_millis_proto(bytes: &[u8]) -> Result<OffsetDateTime, Error> {
    let proto = proto::CurrentTimeInMillis::decode(bytes)?;
    let millis = proto.current_time_in_millis.unwrap_or(0);
    OffsetDateTime::from_unix_timestamp_nanos(millis as i128 * 1_000_000).map_err(|e| Error::parse_proto("current_time_in_millis", e.to_string()))
}

pub(crate) fn decode_managed_accounts(message: &ResponseMessage) -> Result<Vec<String>, Error> {
    decode_managed_accounts_proto(message.require_proto()?)
}

pub(crate) fn decode_managed_accounts_proto(bytes: &[u8]) -> Result<Vec<String>, Error> {
    let p = proto::ManagedAccounts::decode(bytes)?;
    Ok(p.accounts_list
        .as_deref()
        .unwrap_or_default()
        .split(',')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect())
}

pub(crate) fn decode_account_multi_value(message: &ResponseMessage) -> Result<AccountMultiValue, Error> {
    decode_account_multi_value_proto(message.require_proto()?)
}

// === Protobuf decoders ===

pub(crate) fn decode_position_proto(bytes: &[u8]) -> Result<Position, Error> {
    let p = proto::Position::decode(bytes)?;
    let contract = p.contract.as_ref().map(proto::decoders::decode_contract).transpose()?.unwrap_or_default();
    Ok(Position {
        account: p.account.unwrap_or_default(),
        contract,
        position: parse_str_f64(&p.position),
        average_cost: p.avg_cost.unwrap_or_default(),
    })
}

pub(crate) fn decode_account_value_proto(bytes: &[u8]) -> Result<AccountValue, Error> {
    let p = proto::AccountValue::decode(bytes)?;
    Ok(AccountValue {
        key: p.key.unwrap_or_default(),
        value: p.value.unwrap_or_default(),
        currency: p.currency.unwrap_or_default(),
        account: p.account_name,
    })
}

pub(crate) fn decode_account_portfolio_value_proto(bytes: &[u8]) -> Result<AccountPortfolioValue, Error> {
    let p = proto::PortfolioValue::decode(bytes)?;
    let contract = p.contract.as_ref().map(proto::decoders::decode_contract).transpose()?.unwrap_or_default();
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

pub(crate) fn decode_pnl_proto(bytes: &[u8]) -> Result<PnL, Error> {
    let p = proto::PnL::decode(bytes)?;
    Ok(PnL {
        daily_pnl: p.daily_pn_l.unwrap_or_default(),
        unrealized_pnl: proto::decoders::optional_f64(p.unrealized_pn_l),
        realized_pnl: proto::decoders::optional_f64(p.realized_pn_l),
    })
}

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

pub(crate) fn decode_account_summary_proto(bytes: &[u8]) -> Result<AccountSummary, Error> {
    let p = proto::AccountSummary::decode(bytes)?;
    Ok(AccountSummary {
        account: p.account.unwrap_or_default(),
        tag: p.tag.unwrap_or_default(),
        value: p.value.unwrap_or_default(),
        currency: p.currency.unwrap_or_default(),
    })
}

pub(crate) fn decode_account_update_time_proto(bytes: &[u8]) -> Result<AccountUpdateTime, Error> {
    let p = proto::AccountUpdateTime::decode(bytes)?;
    Ok(AccountUpdateTime {
        timestamp: p.time_stamp.unwrap_or_default(),
    })
}

pub(crate) fn decode_position_multi_proto(bytes: &[u8]) -> Result<PositionMulti, Error> {
    let p = proto::PositionMulti::decode(bytes)?;
    let contract = p.contract.as_ref().map(proto::decoders::decode_contract).transpose()?.unwrap_or_default();
    Ok(PositionMulti {
        account: p.account.unwrap_or_default(),
        contract,
        position: parse_str_f64(&p.position),
        average_cost: p.avg_cost.unwrap_or_default(),
        model_code: p.model_code.unwrap_or_default(),
    })
}

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

pub(crate) fn decode_family_codes_proto(bytes: &[u8]) -> Result<Vec<FamilyCode>, Error> {
    let p = proto::FamilyCodes::decode(bytes)?;
    Ok(p.family_codes
        .into_iter()
        .map(|c| FamilyCode {
            account_id: c.account_id.unwrap_or_default(),
            family_code: c.family_code.unwrap_or_default(),
        })
        .collect())
}

pub(crate) fn decode_soft_dollar_tiers(message: &ResponseMessage) -> Result<Vec<SoftDollarTier>, Error> {
    decode_soft_dollar_tiers_proto(message.require_proto()?)
}

pub(crate) fn decode_soft_dollar_tiers_proto(bytes: &[u8]) -> Result<Vec<SoftDollarTier>, Error> {
    let p = proto::SoftDollarTiers::decode(bytes)?;
    Ok(p.soft_dollar_tiers.iter().map(proto::decoders::decode_soft_dollar_tier).collect())
}

pub(in crate::accounts) fn decode_soft_dollar_tiers_message(message: &ResponseMessage) -> Result<Vec<SoftDollarTier>, Error> {
    match message.message_type() {
        IncomingMessages::SoftDollarTier => decode_soft_dollar_tiers(message),
        IncomingMessages::Error => Err(Error::from(message)),
        _ => Err(Error::unexpected_response(message)),
    }
}

pub(crate) fn decode_user_info(message: &ResponseMessage) -> Result<UserInfo, Error> {
    decode_user_info_proto(message.require_proto()?)
}

pub(crate) fn decode_user_info_proto(bytes: &[u8]) -> Result<UserInfo, Error> {
    let p = proto::UserInfo::decode(bytes)?;
    Ok(UserInfo {
        white_branding_id: p.white_branding_id.unwrap_or_default(),
    })
}

pub(in crate::accounts) fn decode_user_info_message(message: &ResponseMessage) -> Result<UserInfo, Error> {
    match message.message_type() {
        IncomingMessages::UserInfo => decode_user_info(message),
        IncomingMessages::Error => Err(Error::from(message)),
        _ => Err(Error::unexpected_response(message)),
    }
}

pub(crate) fn decode_receive_fa(message: &ResponseMessage) -> Result<FaConfig, Error> {
    decode_receive_fa_proto(message.require_proto()?)
}

pub(crate) fn decode_receive_fa_proto(bytes: &[u8]) -> Result<FaConfig, Error> {
    let p = proto::ReceiveFa::decode(bytes)?;
    let _ = p.fa_data_type.map(FaDataType::from_i32).transpose()?;
    Ok(FaConfig {
        xml: p.xml.unwrap_or_default(),
    })
}

pub(crate) fn decode_replace_fa_end(message: &ResponseMessage) -> Result<ReplaceFaResult, Error> {
    decode_replace_fa_end_proto(message.require_proto()?)
}

pub(crate) fn decode_replace_fa_end_proto(bytes: &[u8]) -> Result<ReplaceFaResult, Error> {
    let p = proto::ReplaceFaEnd::decode(bytes)?;
    Ok(ReplaceFaResult {
        text: p.text.unwrap_or_default(),
    })
}

pub(in crate::accounts) fn decode_replace_fa_end_message(message: &ResponseMessage) -> Result<ReplaceFaResult, Error> {
    match message.message_type() {
        IncomingMessages::ReplaceFAEnd => decode_replace_fa_end(message),
        IncomingMessages::Error => Err(Error::from(message)),
        _ => Err(Error::unexpected_response(message)),
    }
}

pub(crate) fn decode_verify_message_api(message: &ResponseMessage) -> Result<VerificationChallenge, Error> {
    decode_verify_message_api_proto(message.require_proto()?)
}

pub(crate) fn decode_verify_message_api_proto(bytes: &[u8]) -> Result<VerificationChallenge, Error> {
    let p = proto::VerifyMessageApi::decode(bytes)?;
    Ok(VerificationChallenge {
        api_data: p.api_data.unwrap_or_default(),
    })
}

pub(crate) fn decode_verify_completed(message: &ResponseMessage) -> Result<VerificationResult, Error> {
    decode_verify_completed_proto(message.require_proto()?)
}

pub(crate) fn decode_verify_completed_proto(bytes: &[u8]) -> Result<VerificationResult, Error> {
    let p = proto::VerifyCompleted::decode(bytes)?;
    Ok(VerificationResult {
        is_successful: p.is_successful.unwrap_or_default(),
        error_text: p.error_text.unwrap_or_default(),
    })
}

/// Dispatch an account-update frame to the right [`AccountUpdate`] variant by
/// `IncomingMessages` type. Used by the connection layer's startup callback path.
pub(crate) fn decode_account_update_message(message: &ResponseMessage) -> Result<AccountUpdate, Error> {
    match message.message_type() {
        IncomingMessages::AccountValue => decode_account_value(message).map(AccountUpdate::AccountValue),
        IncomingMessages::PortfolioValue => decode_account_portfolio_value(message).map(AccountUpdate::PortfolioValue),
        IncomingMessages::AccountUpdateTime => decode_account_update_time(message).map(AccountUpdate::UpdateTime),
        IncomingMessages::AccountDownloadEnd => Ok(AccountUpdate::End),
        _ => Err(Error::unexpected_response(message)),
    }
}

#[cfg(test)]
mod tests;
