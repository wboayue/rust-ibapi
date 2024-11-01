use serde::{Deserialize, Serialize};

use crate::{client::{DataStream, ResponseContext, Subscription}, messages::OutgoingMessages, orders::TagValue, server_versions, Client, Error};

// Requests an XML list of scanner parameters valid in TWS.
pub(super) fn scanner_parameters(client: &Client) -> Result<String, Error> {
    let request = encoders::encode_scanner_parameters()?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestScannerParameters, request)?;
    match subscription.next() {
        Some(Ok(message)) => decoders::decode_scanner_parameters(message),
        Some(Err(Error::ConnectionReset)) => scanner_parameters(client),
        Some(Err(e)) => Err(e),
        None => Err(Error::UnexpectedEndOfStream),
    }
}

pub struct ScannerSubscription {
    /// The number of rows to be returned for the query
    pub number_of_rows: i32,
    /// The instrument's type for the scan. I.e. STK, FUT.HK, etc.
    pub instrument: Option<String>,
    /// The request's location (STK.US, STK.US.MAJOR, etc).
    pub location_code: Option<String>,
    /// Same as TWS Market Scanner's "parameters" field, for example: TOP_PERC_GAIN
    pub scan_code: Option<String>,
    /// Filters out Contracts which price is below this value
    pub above_price: Option<f64>,
    /// Filters out contracts which price is above this value.
    pub below_price: Option<f64>,
    /// Filters out Contracts which volume is above this value.
    pub above_volume: Option<i32>,
    /// Filters out Contracts which option volume is above this value.
    pub average_option_volume_above: Option<i32>,
    /// Filters out Contracts which market cap is above this value.
    pub market_cap_above: Option<f64>,
    /// Filters out Contracts which market cap is below this value.
    pub market_cap_below: Option<f64>,
    /// Filters out Contracts which Moody's rating is below this value.
    pub moody_rating_above: Option<String>,
    /// Filters out Contracts which Moody's rating is above this value.
    pub moody_rating_below: Option<String>,
    /// Filters out Contracts with a S&P rating below this value.
    pub sp_rating_above: Option<String>,
    /// Filters out Contracts with a S&P rating above this value.
    pub sp_rating_below: Option<String>,
    /// Filter out Contracts with a maturity date earlier than this value.
    pub maturity_date_above: Option<String>,
    /// Filter out Contracts with a maturity date older than this value.
    pub maturity_date_below: Option<String>,
    /// Filter out Contracts with a coupon rate lower than this value.
    pub coupon_rate_above: Option<f64>,
    /// Filter out Contracts with a coupon rate higher than this value.
    pub coupon_rate_below: Option<f64>,
    /// Filters out Convertible bonds
    pub exclude_convertible: bool,
    /// For example, a pairing "Annual, true" used on the "top Option Implied Vol % Gainers" scan would return annualized volatilities.
    pub scanner_setting_pairs: Option<String>,
    /// CORP = Corporation, ADR = American Depositary Receipt, ETF = Exchange Traded Fund, REIT = Real Estate Investment Trust, CEF = Closed End Fund
    pub stock_type_filter: Option<String>,
}

impl Default for ScannerSubscription {
    fn default() -> Self {
        ScannerSubscription {
            number_of_rows: -1,
            instrument: None,
            location_code: None,
            scan_code: None,
            above_price: None,
            below_price: None,
            above_volume: None,
            average_option_volume_above: None,
            market_cap_above: None,
            market_cap_below: None,
            moody_rating_above: None,
            moody_rating_below: None,
            sp_rating_above: None,
            sp_rating_below: None,
            maturity_date_above: None,
            maturity_date_below: None,
            coupon_rate_above: None,
            coupon_rate_below: None,
            exclude_convertible: false,
            scanner_setting_pairs: None,
            stock_type_filter: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Scanner {
    Data(ScannerData),
    End,
}

impl DataStream<Scanner> for Scanner {
    fn decode(client: &Client, message: &mut crate::messages::ResponseMessage) -> Result<Scanner, Error> {
        Err(Error::NotImplemented)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
/// Provides the data resulting from the market scanner request.
pub struct ScannerData {
    /// The ranking position of the contract in the scanner sort.
    pub rank: i32,
    /// The contract matching the scanner subscription/
    pub contract: crate::contracts::Contract,
}

pub(super) fn scanner_subscription<'a>(client: &'a Client, subscription: &ScannerSubscription, filter: &Vec<TagValue>) -> Result<Subscription<'a, Scanner>, Error> {
    if !filter.is_empty() {
        client.check_server_version(
            server_versions::SCANNER_GENERIC_OPTS,
            "It does not support API scanner subscription generic filter options.",
        )?
    }

    let request_id = client.next_request_id();
    let request = encoders::encode_scanner_subscription(request_id, client.server_version, subscription, filter)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}

mod encoders {
    use crate::messages::OutgoingMessages;
    use crate::messages::RequestMessage;
    use crate::orders::TagValue;
    use crate::server_versions;
    use crate::Error;

    use super::ScannerSubscription;

    pub(super) fn encode_scanner_parameters() -> Result<RequestMessage, Error> {
        const VERSION: i32 = 1;

        let mut message = RequestMessage::new();

        message.push_field(&OutgoingMessages::RequestScannerParameters);
        message.push_field(&VERSION);

        Ok(message)
    }

    pub(super) fn encode_scanner_subscription(request_id: i32, server_version: i32, subscription: &ScannerSubscription, filter: &Vec<TagValue>) -> Result<RequestMessage, Error> {
        const VERSION: i32 = 4;

        let mut message = RequestMessage::new();

        message.push_field(&OutgoingMessages::RequestScannerParameters);
        if server_version < server_versions::SCANNER_GENERIC_OPTS {
            message.push_field(&VERSION);
        }
        message.push_field(&request_id);
        message.push_field(&subscription.number_of_rows);
        message.push_field(&subscription.instrument);
        message.push_field(&subscription.location_code);
        message.push_field(&subscription.scan_code);

        message.push_field(&subscription.above_price);
        message.push_field(&subscription.below_price);
        message.push_field(&subscription.above_volume);
        message.push_field(&subscription.market_cap_above);
        message.push_field(&subscription.market_cap_below);
        message.push_field(&subscription.moody_rating_above);
        message.push_field(&subscription.moody_rating_below);
        message.push_field(&subscription.sp_rating_above);
        message.push_field(&subscription.sp_rating_below);
        message.push_field(&subscription.maturity_date_above);
        message.push_field(&subscription.maturity_date_below);
        message.push_field(&subscription.coupon_rate_above);
        message.push_field(&subscription.coupon_rate_below);
        message.push_field(&subscription.exclude_convertible);
        message.push_field(&subscription.average_option_volume_above);
        message.push_field(&subscription.scanner_setting_pairs);
        message.push_field(&subscription.stock_type_filter);

        if server_version >= server_versions::SCANNER_GENERIC_OPTS {
            message.push_field(filter);
        }
        if server_version >= server_versions::LINKING {
            message.push_field(&"");    // ignore subscription options
        }

        Ok(message)
    }
}

mod decoders {
    use crate::messages::ResponseMessage;
    use crate::Error;

    pub(super) fn decode_scanner_parameters(mut message: ResponseMessage) -> Result<String, Error> {
        message.skip(); // skip message type
        message.skip(); // skip message version

        Ok(message.next_string()?)
    }
}
