//! Asynchronous implementation of scanner functionality

use super::common::{decoders, encoders};
use super::*;
use crate::messages::OutgoingMessages;
use crate::orders::TagValue;
use crate::subscriptions::Subscription;
use crate::{server_versions, Client, Error};

impl Client {
    /// Requests an XML list of scanner parameters valid in TWS.
    pub async fn scanner_parameters(&self) -> Result<String, Error> {
        let request = encoders::encode_scanner_parameters()?;
        let mut subscription = self.send_shared_request(OutgoingMessages::RequestScannerParameters, request).await?;

        match subscription.next().await {
            Some(Ok(message)) => decoders::decode_scanner_parameters(message),
            Some(Err(e)) => Err(e),
            None => Err(Error::UnexpectedEndOfStream),
        }
    }

    /// Starts a subscription to market scan results based on the provided parameters.
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
            None,
            self.decoder_context(),
        ))
    }
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
