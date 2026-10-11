pub(crate) mod decoders;
pub(crate) mod encoders;
pub(crate) mod tick;

use time::OffsetDateTime;

use crate::market_data::historical::WhatToShow;
use crate::Error;

/// Validate preconditions for historical_data requests.
pub(crate) fn validate_historical_data(end_date: Option<OffsetDateTime>, what_to_show: WhatToShow) -> Result<(), Error> {
    if end_date.is_some() && what_to_show == WhatToShow::AdjustedLast {
        return Err(Error::InvalidArgument(
            "end_date must be None when requesting WhatToShow::AdjustedLast.".into(),
        ));
    }

    Ok(())
}
