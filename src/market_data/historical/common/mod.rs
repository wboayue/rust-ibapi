pub(crate) mod decoders;
pub(crate) mod encoders;

use time::OffsetDateTime;

use crate::contracts::Contract;
use crate::market_data::historical::WhatToShow;
use crate::protocol::{check_version, Features};
use crate::Error;

/// Validate preconditions for historical_data requests.
pub(crate) fn validate_historical_data(
    server_version: i32,
    contract: &Contract,
    end_date: Option<OffsetDateTime>,
    what_to_show: Option<WhatToShow>,
) -> Result<(), Error> {
    if !contract.trading_class.is_empty() || contract.contract_id > 0 {
        check_version(server_version, Features::TRADING_CLASS)?;
    }

    if what_to_show == Some(WhatToShow::Schedule) {
        check_version(server_version, Features::HISTORICAL_SCHEDULE)?;
    }

    if end_date.is_some() && what_to_show == Some(WhatToShow::AdjustedLast) {
        return Err(Error::InvalidArgument(
            "end_date must be None when requesting WhatToShow::AdjustedLast.".into(),
        ));
    }

    Ok(())
}
