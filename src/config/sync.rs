//! Synchronous implementation of configuration retrieval.

use crate::client::sync::Client;
use crate::{
    common::request_helpers,
    protocol::{check_version, Features},
    Error,
};

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
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
