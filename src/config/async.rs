//! Asynchronous implementation of configuration retrieval.

use crate::{
    common::request_helpers,
    protocol::{check_version, Features},
    Client, Error,
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
    /// use ibapi::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let config = client.config().await.expect("request config failed");
    ///     println!("{config:?}");
    /// }
    /// ```
    pub async fn config(&self) -> Result<Config, Error> {
        check_version(self.server_version(), Features::CONFIG)?;

        request_helpers::one_shot_request_with_retry(self, encoders::encode_request_config, decoders::decode_config_message, || {
            Err(Error::UnexpectedEndOfStream)
        })
        .await
    }
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
