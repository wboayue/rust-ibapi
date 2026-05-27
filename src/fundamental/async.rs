//! Asynchronous implementation of fundamental-data functionality.

use crate::common::request_helpers;
use crate::contracts::Contract;
use crate::protocol::{check_version, Features};
use crate::{Client, Error};

use super::{common::decoders, common::encoders, FundamentalData, FundamentalReportType};

impl Client {
    /// Requests a fundamental data report for the given contract.
    ///
    /// The response is a single XML payload (schema varies by report type);
    /// this crate does not parse it. The request is one-shot — if you need to
    /// retrieve a different report, call this method again.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::contracts::Contract;
    /// use ibapi::fundamental::FundamentalReportType;
    /// use ibapi::Client;
    ///
    /// # async fn run() {
    /// let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///
    /// let contract = Contract::stock("AAPL").build();
    /// let report = client
    ///     .fundamental_data(&contract, FundamentalReportType::ReportSnapshot)
    ///     .await
    ///     .expect("fundamental data request failed");
    /// println!("{}", report.data);
    /// # }
    /// ```
    pub async fn fundamental_data(&self, contract: &Contract, report_type: FundamentalReportType) -> Result<FundamentalData, Error> {
        check_version(self.server_version(), Features::FUNDAMENTAL_DATA)?;

        request_helpers::one_shot_request_with_retry(
            self,
            |request_id| encoders::encode_request_fundamental_data(request_id, contract, report_type),
            decoders::decode_fundamental_data_message,
            || Err(Error::UnexpectedEndOfStream),
        )
        .await
    }
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
