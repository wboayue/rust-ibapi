use time::OffsetDateTime;

use crate::messages::ResponseMessage;
use crate::proto::decoders::{required, required_decimal};
use crate::{proto, Error};

use super::super::{
    AccountMultiValue, AccountPortfolioValue, AccountSummary, AccountUpdate, AccountUpdateTime, AccountValue, FaConfig, FaDataType, FamilyCode, PnL,
    PnLSingle, Position, PositionMulti, ReplaceFaResult, UserInfo, VerificationChallenge, VerificationResult,
};
use crate::messages::IncomingMessages;
use crate::orders::SoftDollarTier;

pub(crate) fn decode_position(message: &ResponseMessage) -> Result<Position, Error> {
    decode_position_proto(message.proto()?)
}

pub(crate) fn decode_position_multi(message: &ResponseMessage) -> Result<PositionMulti, Error> {
    decode_position_multi_proto(message.proto()?)
}

pub(crate) fn decode_pnl(message: &ResponseMessage) -> Result<PnL, Error> {
    decode_pnl_proto(message.proto()?)
}

pub(crate) fn decode_pnl_single(message: &ResponseMessage) -> Result<PnLSingle, Error> {
    decode_pnl_single_proto(message.proto()?)
}

pub(crate) fn decode_account_summary(message: &ResponseMessage) -> Result<AccountSummary, Error> {
    decode_account_summary_proto(message.proto()?)
}

pub(crate) fn decode_account_value(message: &ResponseMessage) -> Result<AccountValue, Error> {
    decode_account_value_proto(message.proto()?)
}

pub(crate) fn decode_account_portfolio_value(message: &ResponseMessage) -> Result<AccountPortfolioValue, Error> {
    decode_account_portfolio_value_proto(message.proto()?)
}

pub(crate) fn decode_account_update_time(message: &ResponseMessage) -> Result<AccountUpdateTime, Error> {
    decode_account_update_time_proto(message.proto()?)
}

pub(crate) fn decode_server_time_proto(proto: crate::proto::CurrentTime) -> Result<OffsetDateTime, Error> {
    let timestamp = proto.current_time.unwrap_or(0);
    OffsetDateTime::from_unix_timestamp(timestamp).map_err(|e| Error::parse_proto("current_time", e.to_string()))
}

pub(crate) fn decode_server_time_millis_proto(proto: crate::proto::CurrentTimeInMillis) -> Result<OffsetDateTime, Error> {
    let millis = proto.current_time_in_millis.unwrap_or(0);
    OffsetDateTime::from_unix_timestamp_nanos(millis as i128 * 1_000_000).map_err(|e| Error::parse_proto("current_time_in_millis", e.to_string()))
}

pub(crate) fn decode_managed_accounts_proto(p: crate::proto::ManagedAccounts) -> Result<Vec<String>, Error> {
    Ok(p.accounts_list
        .as_deref()
        .unwrap_or_default()
        .split(',')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect())
}

pub(crate) fn decode_account_multi_value(message: &ResponseMessage) -> Result<AccountMultiValue, Error> {
    decode_account_multi_value_proto(message.proto()?)
}

// === Protobuf decoders ===

pub(crate) fn decode_position_proto(p: crate::proto::Position) -> Result<Position, Error> {
    let contract = proto::decoders::decode_contract(required(p.contract.as_ref(), "contract", "Position")?)?;
    Ok(Position {
        account: p.account.unwrap_or_default(),
        contract,
        position: required_decimal(p.position.as_deref(), "position", "Position")?,
        average_cost: p.avg_cost.unwrap_or_default(),
    })
}

pub(crate) fn decode_account_value_proto(p: crate::proto::AccountValue) -> Result<AccountValue, Error> {
    Ok(AccountValue {
        key: p.key.unwrap_or_default(),
        value: p.value.unwrap_or_default(),
        currency: p.currency.unwrap_or_default(),
        account: p.account_name,
    })
}

pub(crate) fn decode_account_portfolio_value_proto(p: crate::proto::PortfolioValue) -> Result<AccountPortfolioValue, Error> {
    let contract = proto::decoders::decode_contract(required(p.contract.as_ref(), "contract", "PortfolioValue")?)?;
    Ok(AccountPortfolioValue {
        contract,
        position: required_decimal(p.position.as_deref(), "position", "PortfolioValue")?,
        market_price: p.market_price.unwrap_or_default(),
        market_value: p.market_value.unwrap_or_default(),
        average_cost: p.average_cost.unwrap_or_default(),
        unrealized_pnl: p.unrealized_pnl.unwrap_or_default(),
        realized_pnl: p.realized_pnl.unwrap_or_default(),
        account: p.account_name,
    })
}

pub(crate) fn decode_pnl_proto(p: crate::proto::PnL) -> Result<PnL, Error> {
    Ok(PnL {
        daily_pnl: p.daily_pn_l.unwrap_or_default(),
        unrealized_pnl: proto::decoders::optional_f64(p.unrealized_pn_l),
        realized_pnl: proto::decoders::optional_f64(p.realized_pn_l),
    })
}

pub(crate) fn decode_pnl_single_proto(p: crate::proto::PnLSingle) -> Result<PnLSingle, Error> {
    Ok(PnLSingle {
        position: required_decimal(p.position.as_deref(), "position", "PnLSingle")?,
        daily_pnl: p.daily_pn_l.unwrap_or_default(),
        unrealized_pnl: p.unrealized_pn_l.unwrap_or_default(),
        realized_pnl: p.realized_pn_l.unwrap_or_default(),
        value: p.value.unwrap_or_default(),
    })
}

pub(crate) fn decode_account_summary_proto(p: crate::proto::AccountSummary) -> Result<AccountSummary, Error> {
    Ok(AccountSummary {
        account: p.account.unwrap_or_default(),
        tag: p.tag.unwrap_or_default(),
        value: p.value.unwrap_or_default(),
        currency: p.currency.unwrap_or_default(),
    })
}

pub(crate) fn decode_account_update_time_proto(p: crate::proto::AccountUpdateTime) -> Result<AccountUpdateTime, Error> {
    Ok(AccountUpdateTime {
        timestamp: p.time_stamp.unwrap_or_default(),
    })
}

pub(crate) fn decode_position_multi_proto(p: crate::proto::PositionMulti) -> Result<PositionMulti, Error> {
    let contract = proto::decoders::decode_contract(required(p.contract.as_ref(), "contract", "PositionMulti")?)?;
    Ok(PositionMulti {
        account: p.account.unwrap_or_default(),
        contract,
        position: required_decimal(p.position.as_deref(), "position", "PositionMulti")?,
        average_cost: p.avg_cost.unwrap_or_default(),
        model_code: p.model_code.unwrap_or_default(),
    })
}

pub(crate) fn decode_account_multi_value_proto(p: crate::proto::AccountUpdateMulti) -> Result<AccountMultiValue, Error> {
    Ok(AccountMultiValue {
        account: p.account.unwrap_or_default(),
        model_code: p.model_code.unwrap_or_default(),
        key: p.key.unwrap_or_default(),
        value: p.value.unwrap_or_default(),
        currency: p.currency.unwrap_or_default(),
    })
}

pub(crate) fn decode_family_codes_proto(p: crate::proto::FamilyCodes) -> Result<Vec<FamilyCode>, Error> {
    Ok(p.family_codes
        .into_iter()
        .map(|c| FamilyCode {
            account_id: c.account_id.unwrap_or_default(),
            family_code: c.family_code.unwrap_or_default(),
        })
        .collect())
}

pub(crate) fn decode_soft_dollar_tiers_proto(p: crate::proto::SoftDollarTiers) -> Result<Vec<SoftDollarTier>, Error> {
    Ok(p.soft_dollar_tiers.iter().map(proto::decoders::decode_soft_dollar_tier).collect())
}

pub(crate) fn decode_user_info_proto(p: crate::proto::UserInfo) -> Result<UserInfo, Error> {
    Ok(UserInfo {
        white_branding_id: p.white_branding_id.unwrap_or_default(),
    })
}

/// The `ReceiveFA` processor for `request_fa(fa_data_type)`. TWS answers on
/// the shared channel with no request id, so a frame of another dataset is a
/// concurrent `request_fa`'s answer: `Ok(None)`, and the caller reads on.
pub(crate) fn receive_fa_for(fa_data_type: FaDataType) -> impl Fn(&crate::messages::ResponseMessage) -> Result<Option<FaConfig>, Error> {
    let decode = crate::common::request_helpers::expect_proto(decode_receive_fa_proto);
    move |message| {
        let config = decode(message)?;
        Ok((config.fa_data_type == fa_data_type).then_some(config))
    }
}

pub(crate) fn decode_receive_fa_proto(p: crate::proto::ReceiveFa) -> Result<FaConfig, Error> {
    let fa_data_type = p
        .fa_data_type
        .ok_or_else(|| Error::Parse(0, String::new(), "missing fa_data_type in ReceiveFa".into()))
        .and_then(FaDataType::from_i32)?;
    Ok(FaConfig {
        fa_data_type,
        xml: p.xml.unwrap_or_default(),
    })
}

pub(crate) fn decode_replace_fa_end_proto(p: crate::proto::ReplaceFaEnd) -> Result<ReplaceFaResult, Error> {
    Ok(ReplaceFaResult {
        text: p.text.unwrap_or_default(),
    })
}

pub(crate) fn decode_verify_message_api_proto(p: crate::proto::VerifyMessageApi) -> Result<VerificationChallenge, Error> {
    Ok(VerificationChallenge {
        api_data: p.api_data.unwrap_or_default(),
    })
}

pub(crate) fn decode_verify_completed_proto(p: crate::proto::VerifyCompleted) -> Result<VerificationResult, Error> {
    Ok(VerificationResult {
        is_successful: p.is_successful.unwrap_or_default(),
        error_text: p.error_text.unwrap_or_default(),
    })
}

/// Dispatch an account-update frame to the right [`AccountUpdate`] variant by
/// `IncomingMessages` type. Used by the connection layer's startup callback path
/// and by `StreamDecoder<AccountUpdate>`.
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
#[path = "decoders_tests.rs"]
mod tests;
