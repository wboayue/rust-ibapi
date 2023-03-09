use anyhow::Result;
use log::info;

use crate::{client::ResponseMessage, contracts::SecurityType, orders::TagValue, server_versions};

use super::{Contract, ContractDescription, ContractDetails, MarketRule};

pub(crate) fn contract_details(server_version: i32, message: &mut ResponseMessage) -> Result<ContractDetails> {
    message.skip(); // message type

    let mut message_version = 8;
    if server_version < server_versions::SIZE_RULES {
        message_version = message.next_int()?;
    }

    let mut request_id = -1;
    if message_version >= 3 {
        request_id = message.next_int()?;
    }

    info!(
        "request_id: {}, server_version: {}, message_version: {}",
        request_id, server_version, message_version
    );

    let mut contract = ContractDetails::default();

    contract.contract.symbol = message.next_string()?;
    contract.contract.security_type = SecurityType::from(&message.next_string()?);
    read_last_trade_date(&mut contract, &message.next_string()?, false)?;
    contract.contract.strike = message.next_double()?;
    contract.contract.right = message.next_string()?;
    contract.contract.exchange = message.next_string()?;
    contract.contract.currency = message.next_string()?;
    contract.contract.local_symbol = message.next_string()?;
    contract.market_name = message.next_string()?;
    contract.contract.trading_class = message.next_string()?;
    contract.contract.contract_id = message.next_int()?;
    contract.min_tick = message.next_double()?;
    if (server_versions::MD_SIZE_MULTIPLIER..server_versions::SIZE_RULES).contains(&server_version) {
        message.next_int()?; // mdSizeMultiplier no longer used
    }
    contract.contract.multiplier = message.next_string()?;
    contract.order_types = message.next_string()?;
    contract.valid_exchanges = message.next_string()?;
    if message_version >= 2 {
        contract.price_magnifier = message.next_int()?;
    }
    if message_version >= 4 {
        contract.under_contract_id = message.next_int()?;
    }
    if message_version >= 5 {
        //        https://github.com/InteractiveBrokers/tws-api/blob/817a905d52299028ac5af08581c8ffde7644cea9/source/csharpclient/client/EDecoder.cs#L1626
        contract.long_name = message.next_string()?;
        contract.contract.primary_exchange = message.next_string()?;
    }
    if message_version >= 6 {
        contract.contract_month = message.next_string()?;
        contract.industry = message.next_string()?;
        contract.category = message.next_string()?;
        contract.subcategory = message.next_string()?;
        contract.time_zone_id = message.next_string()?;
        contract.trading_hours = message.next_string()?;
        contract.liquid_hours = message.next_string()?;
    }
    if message_version >= 8 {
        contract.ev_rule = message.next_string()?;
        contract.ev_multiplier = message.next_double()?;
    }
    if message_version >= 7 {
        let sec_id_list_count = message.next_int()?;
        for _ in 0..sec_id_list_count {
            let tag = message.next_string()?;
            let value = message.next_string()?;
            contract.sec_id_list.push(TagValue { tag, value });
        }
    }
    if server_version > server_versions::AGG_GROUP {
        contract.agg_group = message.next_int()?;
    }
    if server_version > server_versions::UNDERLYING_INFO {
        contract.under_symbol = message.next_string()?;
        contract.under_security_type = message.next_string()?;
    }
    if server_version > server_versions::MARKET_RULES {
        contract.market_rule_ids = message.next_string()?;
    }
    if server_version > server_versions::REAL_EXPIRATION_DATE {
        contract.real_expiration_date = message.next_string()?;
    }
    if server_version > server_versions::STOCK_TYPE {
        contract.stock_type = message.next_string()?;
    }
    if (server_versions::FRACTIONAL_SIZE_SUPPORT..server_versions::SIZE_RULES).contains(&server_version) {
        message.next_double()?; // size min tick -- no longer used
    }
    if server_version >= server_versions::SIZE_RULES {
        contract.min_size = message.next_double()?;
        contract.size_increment = message.next_double()?;
        contract.suggested_size_increment = message.next_double()?;
    }

    Ok(contract)
}

fn read_last_trade_date(contract: &mut ContractDetails, last_trade_date_or_contract_month: &str, is_bond: bool) -> Result<()> {
    if last_trade_date_or_contract_month.is_empty() {
        return Ok(());
    }

    let splitted: Vec<&str> = if last_trade_date_or_contract_month.contains('-') {
        last_trade_date_or_contract_month.split('-').collect()
    } else {
        // let re = Regex::new(r"\s+").unwrap();
        last_trade_date_or_contract_month.split(' ').collect()
    };

    if !splitted.is_empty() {
        if is_bond {
            contract.maturity = splitted[0].to_string();
        } else {
            contract.contract.last_trade_date_or_contract_month = splitted[0].to_string();
        }
    }
    if splitted.len() > 1 {
        contract.last_trade_time = splitted[1].to_string();
    }
    if is_bond && splitted.len() > 2 {
        contract.time_zone_id = splitted[2].to_string();
    }

    Ok(())
}

pub(crate) fn contract_descriptions(server_version: i32, message: &mut ResponseMessage) -> Result<Vec<ContractDescription>> {
    message.skip(); // message type

    let _request_id = message.next_int()?;
    let contract_descriptions_count = message.next_int()?;

    if contract_descriptions_count < 1 {
        return Ok(Vec::default());
    }

    let mut contract_descriptions: Vec<ContractDescription> = Vec::with_capacity(contract_descriptions_count as usize);

    for _ in 0..contract_descriptions_count {
        let mut contract = Contract {
            contract_id: message.next_int()?,
            symbol: message.next_string()?,
            security_type: SecurityType::from(&message.next_string()?),
            primary_exchange: message.next_string()?,
            currency: message.next_string()?,
            ..Default::default()
        };

        let derivative_security_types_count = message.next_int()?;
        let mut derivative_security_types: Vec<String> = Vec::with_capacity(derivative_security_types_count as usize);
        for _ in 0..derivative_security_types_count {
            derivative_security_types.push(message.next_string()?);
        }

        if server_version >= server_versions::BOND_ISSUERID {
            contract.description = message.next_string()?;
            contract.issuer_id = message.next_string()?;
        }

        contract_descriptions.push(ContractDescription {
            contract,
            derivative_security_types,
        });
    }

    Ok(contract_descriptions)
}

pub(crate) fn market_rule(server_version: i32, message: &mut ResponseMessage) -> Result<MarketRule> {
    message.skip(); // message type

    let market_rule_id = message.next_int()?;

    // int marketRuleId = ReadInt();
    // PriceIncrement[] priceIncrements = new PriceIncrement[0];
    // int nPriceIncrements = ReadInt();

// "93", "26", "1", "0", "0.01", ""

    // if (nPriceIncrements > 0)
    // {
    //     Array.Resize(ref priceIncrements, nPriceIncrements);

    //     for (int i = 0; i < nPriceIncrements; ++i)
    //     {
    //         priceIncrements[i] = new PriceIncrement(ReadDouble(), ReadDouble());
    //     }
    // }

    // eWrapper.marketRule(marketRuleId, priceIncrements);

    message.skip(); // message type
    Ok(MarketRule::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_market_rule() {
        let mut message = ResponseMessage::from("12\012\0ad\0");
    
        
        market_rule(12, &mut message);
    }    
}