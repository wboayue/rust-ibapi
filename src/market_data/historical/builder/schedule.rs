use time::OffsetDateTime;

use crate::contracts::Contract;
use crate::market_data::historical::{Duration, Schedule};
use crate::Error;

#[cfg(test)]
#[path = "schedule_tests.rs"]
mod tests;

/// Builder for the historical-schedule API.
#[must_use = "HistoricalScheduleBuilder does nothing until you call .fetch()"]
pub struct HistoricalScheduleBuilder<'a, C> {
    client: &'a C,
    contract: &'a Contract,
    duration: Duration,
    ending: Option<OffsetDateTime>,
}

impl<'a, C> HistoricalScheduleBuilder<'a, C> {
    pub(crate) fn new(client: &'a C, contract: &'a Contract, duration: Duration) -> Self {
        Self {
            client,
            contract,
            duration,
            ending: None,
        }
    }

    /// Anchor the schedule to a specific end date (defaults to now).
    pub fn ending(mut self, end_date: OffsetDateTime) -> Self {
        self.ending = Some(end_date);
        self
    }
}

#[cfg(feature = "sync")]
impl<'a> HistoricalScheduleBuilder<'a, crate::client::sync::Client> {
    /// Submit the request and return the [`Schedule`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::client::blocking::Client;
    /// use ibapi::contracts::Contract;
    /// use ibapi::market_data::historical::ToDuration;
    /// use time::macros::datetime;
    ///
    /// let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    /// let contract = Contract::stock("GM").build();
    ///
    /// // Ending at a specific date:
    /// let schedule = client
    ///     .historical_schedules(&contract, 30.days())
    ///     .ending(datetime!(2023-04-15 0:00 UTC))
    ///     .fetch()
    ///     .expect("historical schedule request failed");
    ///
    /// // Ending now (no `.ending()`):
    /// let schedule = client
    ///     .historical_schedules(&contract, 30.days())
    ///     .fetch()
    ///     .expect("historical schedule request failed");
    ///
    /// for session in &schedule.sessions {
    ///     println!("{session:?}");
    /// }
    /// ```
    pub fn fetch(self) -> Result<Schedule, Error> {
        crate::market_data::historical::sync::historical_schedule(self.client, self.contract, self.ending, self.duration)
    }
}

#[cfg(feature = "async")]
impl<'a> HistoricalScheduleBuilder<'a, crate::client::r#async::Client> {
    /// Submit the request and return the [`Schedule`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ibapi::prelude::*;
    /// use time::macros::datetime;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");
    ///     let contract = Contract::stock("GM").build();
    ///
    ///     // Ending at a specific date:
    ///     let schedule = client
    ///         .historical_schedules(&contract, 30.days())
    ///         .ending(datetime!(2023-04-15 0:00 UTC))
    ///         .fetch()
    ///         .await
    ///         .expect("historical schedule request failed");
    ///
    ///     for session in &schedule.sessions {
    ///         println!("{session:?}");
    ///     }
    /// }
    /// ```
    pub async fn fetch(self) -> Result<Schedule, Error> {
        crate::market_data::historical::r#async::historical_schedule(self.client, self.contract, self.ending, self.duration).await
    }
}
