use std::fmt::Debug;

use anyhow::{anyhow, Result};

use crate::client::Client;
use crate::client::RequestPacket;
use crate::domain::Contract;
use crate::domain::ContractDetails;
use crate::domain::DeltaNeutralContract;
use crate::domain::SecurityType;
use crate::server_versions;
use crate::outgoing_messages;

pub fn stock(symbol: &str) -> Contract {
    Contract {
        symbol: symbol.to_string(),
        ..default()
    }
}

pub fn default() -> Contract {
    Contract {
        contract_id: 1,
        symbol: "".to_string(),
        security_type: SecurityType::STK,
        last_trade_date_or_contract_month: "123".to_string(),
        strike: 0.0,
        right: "".to_string(),
        multiplier: "".to_string(),
        exchange: "".to_string(),
        currency: "".to_string(),
        local_symbol: "".to_string(),
        primary_exchange: "".to_string(),
        trading_class: "".to_string(),
        include_expired: true,
        security_id_type: "".to_string(),
        security_id: "".to_string(),
        combo_legs_description: "".to_string(),
        combo_legs: Vec::new(),
        issuer_id: "".to_string(),
        delta_neutral_contract: DeltaNeutralContract {
            contract_id: "".to_string(),
            delta: 1.0,
            price: 12.0,
        },
    }
}

/// Requests contract information.
/// This method will provide all the contracts matching the contract provided. It can also be used to retrieve complete options and futures chains. This information will be returned at EWrapper:contractDetails. Though it is now (in API version > 9.72.12) advised to use reqSecDefOptParams for that purpose.
pub fn contract_details<C: Client + Debug>(
    client: &mut C,
    contract: &Contract,
) -> Result<ContractDetails> {
    if !contract.security_id_type.is_empty() || !contract.security_id.is_empty() {
        client.check_server_version(server_versions::SEC_ID_TYPE, "It does not support secIdType not secId attributes")?
    }

    if !contract.trading_class.is_empty() {
        client.check_server_version(server_versions::TRADING_CLASS, "It does not support the TradingClass parameter when requesting contract details.")?
    }

    if !contract.primary_exchange.is_empty() {
        client.check_server_version(server_versions::LINKING, "It does not support PrimaryExch parameter when requesting contract details.")?
    }

    if !contract.issuer_id.is_empty() {
        client.check_server_version(server_versions::BOND_ISSUERID, "It does not support IssuerId parameter when requesting contract details.")?
    }

    const VERSION: i32 = 8;

    let mut packet = RequestPacket::default();

    packet.add_field(outgoing_messages::REQUEST_CONTRACT_DATA);
    packet.add_field(VERSION);

    // if (serverVersion >= MinServerVer.CONTRACT_DATA_CHAIN)
    // {
    //     paramsList.AddParameter(reqId);
    // }
    // if (serverVersion >= MinServerVer.CONTRACT_CONID)
    // {
    //     paramsList.AddParameter(contract.ConId);
    // }
    // paramsList.AddParameter(contract.Symbol);
    // paramsList.AddParameter(contract.SecType);
    // paramsList.AddParameter(contract.LastTradeDateOrContractMonth);
    // paramsList.AddParameter(contract.Strike);
    // paramsList.AddParameter(contract.Right);
    // if (serverVersion >= 15)
    // {
    //     paramsList.AddParameter(contract.Multiplier);
    // }

    // if (serverVersion >= MinServerVer.PRIMARYEXCH)
    // {
    //     paramsList.AddParameter(contract.Exchange);
    //     paramsList.AddParameter(contract.PrimaryExch);
    // }
    // else if (serverVersion >= MinServerVer.LINKING)
    // {
    //     if (!IsEmpty(contract.PrimaryExch) && (contract.Exchange == "BEST" || contract.Exchange == "SMART"))
    //     {
    //         paramsList.AddParameter(contract.Exchange + ":" + contract.PrimaryExch);
    //     }
    //     else
    //     {
    //         paramsList.AddParameter(contract.Exchange);
    //     }
    // }

    // paramsList.AddParameter(contract.Currency);
    // paramsList.AddParameter(contract.LocalSymbol);
    // if (serverVersion >= MinServerVer.TRADING_CLASS)
    // {
    //     paramsList.AddParameter(contract.TradingClass);
    // }
    // if (serverVersion >= 31)
    // {
    //     paramsList.AddParameter(contract.IncludeExpired);
    // }
    // if (serverVersion >= MinServerVer.SEC_ID_TYPE)
    // {
    //     paramsList.AddParameter(contract.SecIdType);
    //     paramsList.AddParameter(contract.SecId);
    // }
    // if (serverVersion >= MinServerVer.MIN_SERVER_VER_BOND_ISSUERID)
    // {
    //     paramsList.AddParameter(contract.IssuerId);
    // }


    client.send_packet(&packet)?;

    print!("{:?} {:?}", client, contract);
    
    Ok(ContractDetails::default())
}
