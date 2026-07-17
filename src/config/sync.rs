//! Synchronous implementation of configuration retrieval.

use crate::client::sync::Client;
use crate::{
    common::request_helpers,
    protocol::{check_version, Features},
    Error,
};

use super::builder::UpdateConfigBuilder;
use super::{common::decoders, encoders, Config};

impl Client {
    /// Reads the TWS/Gateway configuration (API, precautions, orders, and
    /// lock-and-exit settings) the gateway is currently running with.
    ///
    /// This is a read-only snapshot; fields the gateway does not report are
    /// left as `None`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let config = client.config().expect("request config failed");
    /// println!("{config:?}");
    /// ```
    pub fn config(&self) -> Result<Config, Error> {
        check_version(self.server_version, Features::CONFIG)?;

        request_helpers::blocking::one_shot_request_with_retry(self, encoders::encode_request_config, decoders::decode_config_message, || {
            Err(Error::UnexpectedEndOfStream)
        })
    }

    /// Begins a fluent [`UpdateConfigBuilder`] to edit the TWS/Gateway
    /// configuration. Set only the groups you want to change and terminate with
    /// [`submit`](UpdateConfigBuilder::submit).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::config::{OrdersConfig, OrdersSmartRouting};
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    ///
    /// let response = client
    ///     .update_config()
    ///     .orders(OrdersConfig {
    ///         smart_routing: Some(OrdersSmartRouting {
    ///             seek_price_improvement: Some(true),
    ///             ..Default::default()
    ///         }),
    ///     })
    ///     .submit()
    ///     .expect("update config failed");
    /// println!("{response:?}");
    /// ```
    pub fn update_config(&self) -> UpdateConfigBuilder<'_, Client> {
        UpdateConfigBuilder::new(self)
    }
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
