use crate::contracts::Contract;
use crate::protocol::{check_version, Features};
use crate::Error;

pub(crate) fn verify_contract(server_version: i32, contract: &Contract) -> Result<(), Error> {
    if !contract.security_id_type.is_empty() || !contract.security_id.is_empty() {
        check_version(server_version, Features::SEC_ID_TYPE)?
    }

    if !contract.trading_class.is_empty() {
        check_version(server_version, Features::TRADING_CLASS)?
    }

    if !contract.primary_exchange.is_empty() {
        check_version(server_version, Features::LINKING)?
    }

    if !contract.issuer_id.is_empty() {
        check_version(server_version, Features::BOND_ISSUERID)?
    }

    Ok(())
}
