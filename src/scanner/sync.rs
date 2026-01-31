//! Synchronous implementation of scanner functionality

use std::sync::Arc;

use super::common::{decoders, encoders};
use super::*;
use crate::client::blocking::Subscription;
use crate::messages::{OutgoingMessages, RequestMessage, ResponseMessage};
use crate::orders::TagValue;
use crate::subscriptions::{DecoderContext, StreamDecoder};
use crate::{client::sync::Client, server_versions, Error};

impl StreamDecoder<Vec<ScannerData>> for Vec<ScannerData> {
    fn decode(_context: &DecoderContext, message: &mut ResponseMessage) -> Result<Vec<ScannerData>, Error> {
        decoders::decode_scanner_message(message)
    }

    fn cancel_message(_server_version: i32, request_id: Option<i32>, _context: Option<&DecoderContext>) -> Result<RequestMessage, Error> {
        let request_id = request_id.expect("Request ID required to encode cancel scanner subscription.");
        encoders::encode_cancel_scanner_subscription(request_id)
    }
}

/// Requests an XML list of scanner parameters valid in TWS.
pub(crate) fn scanner_parameters(client: &Client) -> Result<String, Error> {
    let request = encoders::encode_scanner_parameters()?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestScannerParameters, request)?;
    match subscription.next() {
        Some(Ok(message)) => decoders::decode_scanner_parameters(message),
        Some(Err(Error::ConnectionReset)) => scanner_parameters(client),
        Some(Err(e)) => Err(e),
        None => Err(Error::UnexpectedEndOfStream),
    }
}

/// Starts a subscription to market scan results based on the provided parameters.
pub(crate) fn scanner_subscription(
    client: &Client,
    subscription: &ScannerSubscription,
    filter: &Vec<TagValue>,
) -> Result<Subscription<Vec<ScannerData>>, Error> {
    if !filter.is_empty() {
        client.check_server_version(
            server_versions::SCANNER_GENERIC_OPTS,
            "It does not support API scanner subscription generic filter options.",
        )?
    }

    let request_id = client.next_request_id();
    let request = encoders::encode_scanner_subscription(request_id, client.server_version, subscription, filter)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(Arc::clone(&client.message_bus), subscription, client.decoder_context()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{Exchange, SecurityType, Symbol};
    use crate::orders::TagValue;
    use crate::server_versions;
    use crate::stubs::MessageBusStub;
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_scanner_parameters() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "19|2|<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<ScanParameterResponse>\n<InstrumentList>...</InstrumentList>\n</ScanParameterResponse>".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::SCANNER_GENERIC_OPTS);

        let result = client.scanner_parameters();
        assert!(result.is_ok(), "failed to request scanner parameters: {}", result.err().unwrap());

        let request_messages = client.message_bus.request_messages();
        assert_eq!(request_messages[0].encode_simple(), "24|1|");

        let scanner_params = result.unwrap();
        assert!(scanner_params.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(scanner_params.contains("<ScanParameterResponse>"));
        assert!(scanner_params.contains("<InstrumentList>"));
    }

    #[test]
    fn test_scanner_subscription() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "20\03\09000\010\00\0670777621\0SVMH\0STK\0\00\0\0SMART\0USD\0SVMH\0NMS\0NMS\0\0\0\0\01\0536918651\0GTI\0STK\0\00\0\0SMART\0USD\0GTI\0NMS\0NMS\0\0\0\0\02\0526726639\0LITM\0STK\0\00\0\0SMART\0USD\0LITM\0SCM\0SCM\0\0\0\0\03\0504716446\0LCID\0STK\0\00\0\0SMART\0USD\0LCID\0NMS\0NMS\0\0\0\0\04\0547605251\0RGTI\0STK\0\00\0\0SMART\0USD\0RGTI\0SCM\0SCM\0\0\0\0\05\0653568762\0AVGR\0STK\0\00\0\0SMART\0USD\0AVGR\0SCM\0SCM\0\0\0\0\06\04815747\0NVDA\0STK\0\00\0\0SMART\0USD\0NVDA\0NMS\0NMS\0\0\0\0\07\0534453483\0HOUR\0STK\0\00\0\0SMART\0USD\0HOUR\0SCM\0SCM\0\0\0\0\08\0631370187\0LAES\0STK\0\00\0\0SMART\0USD\0LAES\0SCM\0SCM\0\0\0\0\09\0689954925\0XTIA\0STK\0\00\0\0SMART\0USD\0XTIA\0SCM\0SCM\0\0\0\0\0".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, server_versions::SCANNER_GENERIC_OPTS);

        let subscription = ScannerSubscription {
            number_of_rows: 10,
            instrument: Some("FUT".to_string()),
            location_code: Some("FUT.US".to_string()),
            scan_code: Some("TOP_PERC_GAIN".to_string()),
            above_price: Some(50.0),
            below_price: Some(100.0),
            above_volume: Some(1000),
            average_option_volume_above: Some(100),
            market_cap_above: Some(1000000.0),
            market_cap_below: Some(10000000.0),
            moody_rating_above: Some("A".to_string()),
            moody_rating_below: Some("AAA".to_string()),
            sp_rating_above: Some("A".to_string()),
            sp_rating_below: Some("AAA".to_string()),
            maturity_date_above: Some("20230101".to_string()),
            maturity_date_below: Some("20231231".to_string()),
            coupon_rate_above: Some(2.0),
            coupon_rate_below: Some(5.0),
            exclude_convertible: true,
            scanner_setting_pairs: Some("Annual,true".to_string()),
            stock_type_filter: Some("CORP".to_string()),
        };

        let filter = vec![
            TagValue {
                tag: "scannerType".to_string(),
                value: "TOP_PERC_GAIN".to_string(),
            },
            TagValue {
                tag: "numberOfRows".to_string(),
                value: "10".to_string(),
            },
        ];

        let result = client.scanner_subscription(&subscription, &filter);
        assert!(result.is_ok(), "failed to request scanner subscription: {}", result.err().unwrap());

        // Now verify we can parse the scanner data responses
        let subscription = result.unwrap();
        let scanner_data: Vec<Vec<ScannerData>> = subscription.iter().collect();

        assert!(
            subscription.error().is_none(),
            "error getting scanner results: {}",
            subscription.error().unwrap()
        );
        assert_eq!(scanner_data.len(), 1);

        // Verify first scanner data entry
        let first = &scanner_data[0][0];
        assert_eq!(first.rank, 0);
        assert_eq!(first.contract_details.contract.symbol, Symbol::from("SVMH"));
        assert_eq!(first.contract_details.contract.security_type, SecurityType::Stock);
        assert_eq!(first.contract_details.contract.exchange, Exchange::from("SMART"));

        // Verify second scanner data entry
        let second = &scanner_data[0][1];
        assert_eq!(second.rank, 1);
        assert_eq!(second.contract_details.contract.symbol, Symbol::from("GTI"));
        assert_eq!(second.contract_details.contract.security_type, SecurityType::Stock);
        assert_eq!(second.contract_details.contract.exchange, Exchange::from("SMART"));

        // Verify third scanner data entry
        let third = &scanner_data[0][2];
        assert_eq!(third.rank, 2);
        assert_eq!(third.contract_details.contract.symbol, Symbol::from("LITM"));
        assert_eq!(third.contract_details.contract.security_type, SecurityType::Stock);
        assert_eq!(third.contract_details.contract.exchange, Exchange::from("SMART"));

        // drop subscription to generate cancel request
        drop(subscription);

        let request_messages = client.message_bus.request_messages();

        // Verify request parameters were encoded correctly
        let scanner_request = format!(
            "22|9000|10|FUT|FUT.US|TOP_PERC_GAIN|50|100|1000|1000000|10000000|A|AAA|A|AAA|20230101|20231231|2|5|1|100|Annual,true|CORP|scannerType=TOP_PERC_GAIN;numberOfRows=10;||",
        );
        assert_eq!(request_messages[0].encode_simple(), scanner_request);

        // Verify cancel request was sent
        assert_eq!(request_messages[1].encode_simple(), "23|1|9000|");
    }
}
