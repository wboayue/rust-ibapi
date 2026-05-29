//! Asynchronous implementation of scanner functionality

use super::common::{decoders, encoders};
use super::*;
use crate::contracts::TagValue;
use crate::messages::OutgoingMessages;
use crate::subscriptions::Subscription;
use crate::{server_versions, Client, Error};

impl Client {
    /// Requests an XML list of scanner parameters valid in TWS.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let xml = client.scanner_parameters().await.expect("scanner_parameters failed");
    ///     println!("scanner parameters: {} chars", xml.len());
    /// }
    /// ```
    pub async fn scanner_parameters(&self) -> Result<String, Error> {
        let request = encoders::encode_scanner_parameters()?;
        let mut subscription = self.send_shared_request(OutgoingMessages::RequestScannerParameters, request).await?;

        match subscription.next().await {
            Some(Ok(message)) => decoders::decode_scanner_parameters(&message),
            Some(Err(e)) => Err(e),
            None => Err(Error::UnexpectedEndOfStream),
        }
    }

    /// Starts a subscription to market scan results based on the provided parameters.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    /// use ibapi::scanner::ScannerSubscription;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    ///     let mut sub = ScannerSubscription::default();
    ///     sub.instrument = Some("STK".to_string());
    ///     sub.location_code = Some("STK.US.MAJOR".to_string());
    ///     sub.scan_code = Some("TOP_PERC_GAIN".to_string());
    ///
    ///     let filter: Vec<ibapi::contracts::TagValue> = Vec::new();
    ///
    ///     let subscription = client
    ///         .scanner_subscription(&sub, &filter)
    ///         .await
    ///         .expect("scanner_subscription failed");
    ///
    ///     // Take the first batch of results, if any.
    ///     let mut data = subscription.filter_data();
    ///     if let Some(batch) = data.next().await {
    ///         match batch {
    ///             Ok(rows) => {
    ///                 for row in rows {
    ///                     println!(
    ///                         "rank: {}, symbol: {}",
    ///                         row.rank, row.contract_details.contract.symbol
    ///                     );
    ///                 }
    ///             }
    ///             Err(e) => eprintln!("scanner error: {e:?}"),
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn scanner_subscription(
        &self,
        subscription: &ScannerSubscription,
        filter: &[TagValue],
    ) -> Result<Subscription<Vec<ScannerData>>, Error> {
        if !filter.is_empty() {
            self.check_server_version(
                server_versions::SCANNER_GENERIC_OPTS,
                "It does not support API scanner subscription generic filter options.",
            )?
        }

        let request_id = self.next_request_id();
        let request = encoders::encode_scanner_subscription(request_id, subscription, filter)?;
        let internal_subscription = self.send_request(request_id, request).await?;

        Ok(Subscription::new_from_internal::<Vec<ScannerData>>(
            internal_subscription,
            self.message_bus.clone(),
            Some(request_id),
            None,
            self.decoder_context(),
        ))
    }
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
