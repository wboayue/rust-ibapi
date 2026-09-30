//! Blocking owned contract enumeration. Read deadlines do not bound writes.

use super::{QueryCore, QueryDisposition, QueryPlan};
use crate::client::sync::Client;
use crate::subscriptions::SubscriptionItem;
use crate::Error;
use std::time::Instant;

/// Blocking counterpart of the async owned enumeration.
///
/// Dropping an unfinished submitted query requests shutdown of this client
/// session (including account subscriptions sharing it), never the Gateway or
/// other clients. Call `Client::disconnect` when joined teardown is required.
///
/// `next_until` and `drain_until` bound local waits. `start` and `request_cancel`
/// are ordinary blocking socket writes: their duration is **not** bounded by
/// those read deadlines. Use the async API for a cancellable total write/read
/// budget. Neither API bounds server work already requested or retries for you.
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "sync")]
/// # fn example(client: &ibapi::client::blocking::Client) -> Result<(), ibapi::Error> {
/// use ibapi::contracts::{Contract, QueryLimits};
/// let mut query = client.prepare_contract_details(&Contract::stock("AAPL").build(), QueryLimits::default())?;
/// println!("request {}", query.request_id());
/// query.start()?;
/// let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
/// while let Some(item) = query.next_until(deadline)? { println!("{item:?}"); }
/// # Ok(()) }
/// ```
#[must_use = "a prepared query does nothing until start; dropping an unfinished submitted query retires its session"]
pub struct ContractQuery<'a, T> {
    client: &'a Client,
    core: QueryCore<T>,
}

impl<'a, T> ContractQuery<'a, T> {
    pub(crate) fn prepare(client: &'a Client, plan: QueryPlan<T>) -> Result<Self, Error> {
        let read = client.message_bus.register_bounded(plan.id, plan.spec)?;
        Ok(Self {
            client,
            core: plan.core(read),
        })
    }

    /// The allocated ID, visible before submission; see the type example.
    pub fn request_id(&self) -> i32 {
        self.core.plan.id
    }
    /// Lifecycle evidence, not proof that collection was complete.
    pub fn disposition(&self) -> QueryDisposition {
        self.core.disposition()
    }

    /// Write exactly once; see the type example. Blocking write duration is
    /// not bounded by a subsequent read deadline. Failure retires this session,
    /// except a refusal while the session is reconnecting: the bus returns
    /// `Error::ConnectionReset` before sending anything, the query stays
    /// `NotSubmitted`, and `start` may be called again once the client has
    /// reconnected (`Client::is_connected`, or the reconnect notice).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "sync")]
    /// # fn submit<T>(query: &mut ibapi::client::blocking::ContractQuery<'_, T>) -> Result<(), ibapi::Error> {
    /// query.start()?;
    /// println!("submitted request {}", query.request_id());
    /// # Ok(()) }
    /// ```
    pub fn start(&mut self) -> Result<(), Error> {
        self.core.begin()?;
        match self.client.message_bus.send_bounded(&self.core.plan.packet) {
            Ok(()) => {}
            Err(Error::ConnectionReset) => {
                // The send gate refused before writing: nothing to retire.
                self.core.unsubmit();
                return Err(Error::ConnectionReset);
            }
            Err(error) => {
                self.core.retired = true;
                self.client.message_bus.request_shutdown_sync();
                return Err(error);
            }
        }
        self.core.uncertain = false;
        Ok(())
    }

    /// Yield a row/notice within the absolute read deadline. Timeout keeps
    /// ownership for cancel/drain; drop retires if completion stays unconfirmed.
    /// A native empty end returns `None`; after an error reads are fused.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "sync")]
    /// # fn read<T: std::fmt::Debug>(query: &mut ibapi::client::blocking::ContractQuery<'_, T>) -> Result<(), ibapi::Error> {
    /// let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    /// while let Some(item) = query.next_until(deadline)? { println!("{item:?}"); }
    /// println!("lifecycle: {:?}", query.disposition());
    /// # Ok(()) }
    /// ```
    pub fn next_until(&mut self, deadline: Instant) -> Result<Option<SubscriptionItem<T>>, Error> {
        if !self.core.before_read()? {
            return Ok(None);
        }
        let item = match self.core.read.next_until(deadline) {
            Ok(item) => item,
            Err(error) => {
                self.core.read_stopped = true;
                return Err(error);
            }
        };
        self.core.accept(item)
    }

    /// Write native cancellation once on contract-details servers 215+.
    /// `true` means written, not acknowledged. `false` means no native cancel
    /// exists or none is needed. This blocking write has no read deadline.
    /// Retain the handle and call `drain_until`, or drop it to retire.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "sync")]
    /// # fn cleanup<T>(query: &mut ibapi::client::blocking::ContractQuery<'_, T>) -> Result<(), ibapi::Error> {
    /// query.request_cancel()?;
    /// let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
    /// let disposition = query.drain_until(deadline)?;
    /// println!("{disposition:?}");
    /// # Ok(()) }
    /// ```
    pub fn request_cancel(&mut self) -> Result<bool, Error> {
        match self.disposition() {
            QueryDisposition::NotSubmitted | QueryDisposition::ResponseEnded | QueryDisposition::DefinitionRejected => return Ok(false),
            QueryDisposition::RetireRequired => return Err(Error::Shutdown),
            QueryDisposition::Pending => {}
        }
        let Some(packet) = self.core.cancel_packet(self.client.server_version())? else {
            return Ok(false);
        };
        if self.core.cancel_attempted {
            return Err(Error::InvalidArgument("native cancel already attempted".into()));
        }
        self.core.cancel_attempted = true;
        self.core.uncertain = true;
        if let Err(error) = self.client.message_bus.send_bounded(&packet) {
            self.core.retired = true;
            self.client.message_bus.request_shutdown_sync();
            return Err(error);
        }
        self.core.uncertain = false;
        Ok(true)
    }

    /// Discard unread rows and await only native terminal evidence. Does not
    /// send cancellation. Timeout/interruption retires this client session.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "sync")]
    /// # fn drain<T>(query: &mut ibapi::client::blocking::ContractQuery<'_, T>) -> Result<(), ibapi::Error> {
    /// let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
    /// println!("cleanup: {:?}", query.drain_until(deadline)?);
    /// # Ok(()) }
    /// ```
    pub fn drain_until(&mut self, deadline: Instant) -> Result<QueryDisposition, Error> {
        if self.disposition() == QueryDisposition::NotSubmitted {
            return Ok(QueryDisposition::NotSubmitted);
        }
        let outcome = if self.core.retired || self.core.uncertain {
            Err(Error::Shutdown)
        } else {
            self.core.read_stopped = true;
            self.core.read.discard_buffered();
            self.core.read.terminal_until(deadline).and_then(super::terminal_disposition)
        };
        if outcome.is_err() {
            self.core.retired = true;
            self.client.message_bus.request_shutdown_sync();
        }
        outcome
    }
}

impl<T> Drop for ContractQuery<'_, T> {
    fn drop(&mut self) {
        if matches!(self.disposition(), QueryDisposition::Pending | QueryDisposition::RetireRequired) {
            self.client.message_bus.request_shutdown_sync();
        }
    }
}
