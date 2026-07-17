//! Synchronous `submit()` for the update-config builder.

use super::update_config_builder::UpdateConfigBuilder;
use crate::client::sync::Client;
use crate::common::request_helpers;
use crate::config::common::{decoders, encoders};
use crate::config::UpdateConfigResponse;
use crate::protocol::{check_version, Features};
use crate::Error;

impl UpdateConfigBuilder<'_, Client> {
    /// Submit the configuration edit synchronously.
    ///
    /// Returns the gateway's [`UpdateConfigResponse`]. If its `warnings` are
    /// non-empty the edit was not applied — re-submit with each warning passed
    /// to [`accept_warning`](UpdateConfigBuilder::accept_warning).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::config::{ApiConfig, ApiSettings};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let response = client
    ///     .update_config()
    ///     .api(ApiConfig {
    ///         settings: Some(ApiSettings {
    ///             socket_port: Some(7497),
    ///             ..Default::default()
    ///         }),
    ///         ..Default::default()
    ///     })
    ///     .submit()
    ///     .expect("update config failed");
    /// println!("{response:?}");
    /// ```
    pub fn submit(self) -> Result<UpdateConfigResponse, Error> {
        check_version(self.client.server_version, Features::UPDATE_CONFIG)?;

        request_helpers::blocking::one_shot_request_with_retry(
            self.client,
            |request_id| encoders::encode_update_config(&self.to_proto(request_id)),
            decoders::decode_update_config_message,
            || Err(Error::UnexpectedEndOfStream),
        )
    }
}

#[cfg(test)]
#[path = "sync_impl_tests.rs"]
mod tests;
