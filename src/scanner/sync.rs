//! Synchronous implementation of scanner functionality

use std::sync::Arc;

use super::common::{decoders, encoders};
use super::*;
use crate::client::blocking::Subscription;
use crate::client::sync::Client;
use crate::common::request_helpers::{self, expect_proto};
use crate::contracts::TagValue;
use crate::messages::{IncomingMessages, OutgoingMessages};
use crate::{server_versions, Error};

impl Client {
    /// Requests an XML list of scanner parameters valid in TWS.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::scanner::ScannerSubscription;
    /// use ibapi::contracts::TagValue;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let mut sub = ScannerSubscription::default();
    /// sub.instrument = Some("STK".to_string());
    /// sub.location_code = Some("STK.US.MAJOR".to_string());
    /// sub.scan_code = Some("TOP_PERC_GAIN".to_string());
    /// // Further customize the subscription object as needed, for example:
    /// // sub.above_price = Some(1.0);
    /// // sub.below_price = Some(100.0);
    /// // sub.number_of_rows = Some(20);
    ///
    /// // Filter options are advanced and not always needed. Pass an empty Vec if not used.
    /// let filter_options: Vec<TagValue> = Vec::new();
    /// // Example of adding a filter:
    /// // filter_options.push(TagValue { tag: "marketCapAbove".to_string(), value: "1000000000".to_string() });
    ///
    /// match client.scanner_subscription(&sub, &filter_options) {
    ///     Ok(subscription) => {
    ///         // Iterate over received scanner data.
    ///         // Note: Scanner subscriptions can be continuous or return a snapshot.
    ///         // This example just takes the first batch if available.
    ///         match subscription.iter_data().next() {
    ///             Some(Ok(scanner_results_vec)) => {
    ///                 println!("Scanner Results (first batch):");
    ///                 for data in scanner_results_vec {
    ///                     println!("  Rank: {}, Symbol: {}",
    ///                              data.rank,
    ///                              data.contract_details.contract.symbol);
    ///                 }
    ///             }
    ///             Some(Err(e)) => eprintln!("Scanner error: {e:?}"),
    ///             None => println!("No scanner results received in the first check."),
    ///         }
    ///         // In a real application, you might continuously iterate or handle updates.
    ///         // Remember to cancel the subscription when no longer needed if it's continuous.
    ///         // subscription.cancel();
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Failed to start scanner subscription: {e:?}");
    ///     }
    /// };
    /// ```
    pub fn scanner_parameters(&self) -> Result<String, Error> {
        request_helpers::blocking::one_shot_with_retry(
            self,
            OutgoingMessages::RequestScannerParameters,
            encoders::encode_scanner_parameters,
            expect_proto(IncomingMessages::ScannerParameters, decoders::decode_scanner_parameters_proto),
            || Err(Error::UnexpectedEndOfStream),
        )
    }

    /// Starts a subscription to market scan results based on the provided parameters.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::scanner::ScannerSubscription;
    /// use ibapi::contracts::TagValue;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let mut sub = ScannerSubscription::default();
    /// sub.instrument = Some("STK".to_string());
    /// sub.location_code = Some("STK.US.MAJOR".to_string());
    /// sub.scan_code = Some("TOP_PERC_GAIN".to_string());
    /// // Further customize the subscription object as needed, for example:
    /// // sub.above_price = Some(1.0);
    /// // sub.below_price = Some(100.0);
    /// // sub.number_of_rows = Some(20);
    ///
    /// // Filter options are advanced and not always needed. Pass an empty Vec if not used.
    /// let mut filter_options: Vec<TagValue> = Vec::new();
    /// // Example of adding a filter:
    /// // filter_options.push(TagValue { tag: "marketCapAbove".to_string(), value: "1000000000".to_string() });
    ///
    /// match client.scanner_subscription(&sub, &filter_options) {
    ///     Ok(subscription) => {
    ///         // Iterate over received scanner data.
    ///         // Note: Scanner subscriptions can be continuous or return a snapshot.
    ///         // This example just takes the first batch if available.
    ///         match subscription.iter_data().next() {
    ///             Some(Ok(scanner_results_vec)) => {
    ///                 println!("Scanner Results (first batch):");
    ///                 for data in scanner_results_vec {
    ///                     println!("  Rank: {}, Symbol: {}",
    ///                              data.rank,
    ///                              data.contract_details.contract.symbol);
    ///                 }
    ///             }
    ///             Some(Err(e)) => eprintln!("Scanner error: {e:?}"),
    ///             None => println!("No scanner results received in the first check."),
    ///         }
    ///         // In a real application, you might continuously iterate or handle updates.
    ///         // Remember to cancel the subscription when no longer needed if it's continuous.
    ///         // subscription.cancel();
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Failed to start scanner subscription: {e:?}");
    ///     }
    /// };
    /// ```
    pub fn scanner_subscription(&self, subscription: &ScannerSubscription, filter: &[TagValue]) -> Result<Subscription<Vec<ScannerData>>, Error> {
        if !filter.is_empty() {
            self.check_server_version(
                server_versions::SCANNER_GENERIC_OPTS,
                "It does not support API scanner subscription generic filter options.",
            )?
        }

        let request_id = self.next_request_id();
        let request = encoders::encode_scanner_subscription(request_id, subscription, filter)?;
        let subscription = self.send_request(request_id, request)?;

        Ok(Subscription::new(Arc::clone(&self.message_bus), subscription, self.decoder_context()))
    }
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
