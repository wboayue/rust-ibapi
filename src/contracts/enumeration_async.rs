//! Asynchronous owned contract enumeration.

use super::{QueryCore, QueryDisposition, QueryPlan};
use crate::client::r#async::Client;
use crate::subscriptions::SubscriptionItem;
use crate::transport::r#async::AsyncMessageBus;
use crate::Error;

/// One prepared, single-attempt reference-data query. Not cloneable.
///
/// Dropping a submitted query without a native end (or code 200 rejection)
/// requests shutdown of **this client session**, which may interrupt account
/// subscriptions on that client. It does not stop the Gateway or other clients.
/// Local unregistering and successful cancel writes are not acknowledgements.
/// Call `Client::disconnect` afterward when joined teardown is required.
///
/// Async reads retain their journal if a `next` future is dropped. Dropping an
/// uncertain write or a pending `drain_until` retires the session. Put start,
/// cancel and drain under the same caller-owned deadline; no retries are hidden.
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "async")]
/// # async fn example(client: &ibapi::Client) -> Result<(), ibapi::Error> {
/// use ibapi::contracts::{Contract, QueryLimits};
/// let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
/// let mut query = client.prepare_contract_details(&Contract::stock("AAPL").build(), QueryLimits::default())?;
/// println!("request {}", query.request_id()); // no wire write yet
/// let work = async {
///     query.start().await?;
///     while let Some(item) = query.next().await? { println!("{item:?}"); }
///     Ok::<_, ibapi::Error>(())
/// };
/// match tokio::time::timeout_at(deadline, work).await {
///     Ok(result) => result?,
///     Err(_) => {
///         // Reserve part of the total budget for cancel + drain if desired.
///         // No budget remains here: drop retires an unfinished request.
///     }
/// }
/// drop(query);
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

    /// The allocated ID, available before submission. See the type example.
    pub fn request_id(&self) -> i32 {
        self.core.plan.id
    }

    /// Lifecycle evidence; not proof that collection was complete.
    pub fn disposition(&self) -> QueryDisposition {
        self.core.disposition()
    }

    /// Write exactly once. A failed/dropped write is conservatively uncertain.
    /// Use a caller timeout for queueing and write time; see the type example.
    /// While the session is reconnecting the bus refuses the write with
    /// `Error::ConnectionReset` before sending anything: the query stays
    /// `NotSubmitted`, the session is not retired, and `start` may be called
    /// again once the client has reconnected (`Client::is_connected`, or the
    /// reconnect notice on the notice stream).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn submit<T>(query: &mut ibapi::contracts::ContractQuery<'_, T>) -> Result<(), ibapi::Error> {
    /// query.start().await?;
    /// println!("submitted request {}", query.request_id());
    /// # Ok(()) }
    /// ```
    pub async fn start(&mut self) -> Result<(), Error> {
        self.core.begin()?;
        let mut guard = RetirementGuard::new(self.client.message_bus.as_ref(), &mut self.core.retired);
        match self.client.message_bus.send_bounded(self.core.plan.packet.clone()).await {
            Ok(()) => {}
            Err(Error::ConnectionReset) => {
                // The send gate refused before writing: nothing to retire.
                guard.armed = false;
                drop(guard);
                self.core.unsubmit();
                return Err(Error::ConnectionReset);
            }
            Err(error) => return Err(error),
        }
        self.core.uncertain = false;
        guard.armed = false;
        Ok(())
    }

    /// Yield the next bounded row/notice. A native empty end returns `None`;
    /// errors are returned once, then reads are fused. Inspect `disposition`
    /// separately after an error. Dropping this read future loses no queued row.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn read<T: std::fmt::Debug>(query: &mut ibapi::contracts::ContractQuery<'_, T>) -> Result<(), ibapi::Error> {
    /// while let Some(item) = query.next().await? { println!("{item:?}"); }
    /// println!("lifecycle: {:?}", query.disposition());
    /// # Ok(()) }
    /// ```
    pub async fn next(&mut self) -> Result<Option<SubscriptionItem<T>>, Error> {
        if !self.core.before_read()? {
            return Ok(None);
        }
        let item = self.core.read.next_async().await;
        self.core.accept(item)
    }

    /// Write native contract-details cancellation once when available (server
    /// 215+). `true` means **written**, not acknowledged. `false` means no write:
    /// already terminal/not submitted, or no native cancel for this request or
    /// server version. Keep ownership and call `drain_until` within the same
    /// caller budget, or drop to retire the session.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn cleanup<T>(query: &mut ibapi::contracts::ContractQuery<'_, T>) -> Result<(), ibapi::Error> {
    /// let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(1);
    /// let cleanup = async {
    ///     query.request_cancel().await?;
    ///     query.drain_until(deadline).await
    /// };
    /// let _outcome = tokio::time::timeout_at(deadline, cleanup).await;
    /// # Ok(()) }
    /// ```
    pub async fn request_cancel(&mut self) -> Result<bool, Error> {
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
        let mut guard = RetirementGuard::new(self.client.message_bus.as_ref(), &mut self.core.retired);
        self.client.message_bus.send_bounded(packet).await?;
        self.core.uncertain = false;
        guard.armed = false;
        Ok(true)
    }

    /// Discard unread rows and wait only for the native terminal state. Does
    /// not write a cancel. The absolute deadline is never renewed by traffic.
    /// Timeout, interruption, or dropping this future retires this session.
    /// Previously returned rows/notices remain owned by the caller.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn drain<T>(query: &mut ibapi::contracts::ContractQuery<'_, T>) -> Result<(), ibapi::Error> {
    /// let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(1);
    /// println!("cleanup: {:?}", query.drain_until(deadline).await?);
    /// # Ok(()) }
    /// ```
    pub async fn drain_until(&mut self, deadline: tokio::time::Instant) -> Result<QueryDisposition, Error> {
        if self.disposition() == QueryDisposition::NotSubmitted {
            return Ok(QueryDisposition::NotSubmitted);
        }
        if self.core.retired || self.core.uncertain {
            self.client.message_bus.request_shutdown_sync();
            self.core.retired = true;
            return Err(Error::Shutdown);
        }
        self.core.read_stopped = true;
        self.core.read.discard_buffered();
        let mut guard = RetirementGuard::new(self.client.message_bus.as_ref(), &mut self.core.retired);
        let terminal = match self.core.read.terminal() {
            Some(terminal) => terminal,
            None => tokio::time::timeout_at(deadline, self.core.read.terminal_async())
                .await
                .map_err(|_| Error::Io(std::io::Error::new(std::io::ErrorKind::TimedOut, "bounded query cleanup deadline")))?,
        };
        let disposition = super::terminal_disposition(terminal)?;
        guard.armed = false;
        Ok(disposition)
    }
}

impl<T> Drop for ContractQuery<'_, T> {
    fn drop(&mut self) {
        if matches!(self.disposition(), QueryDisposition::Pending | QueryDisposition::RetireRequired) {
            self.client.message_bus.request_shutdown_sync();
        }
    }
}

struct RetirementGuard<'a> {
    bus: &'a dyn AsyncMessageBus,
    retired: &'a mut bool,
    armed: bool,
}
impl<'a> RetirementGuard<'a> {
    fn new(bus: &'a dyn AsyncMessageBus, retired: &'a mut bool) -> Self {
        Self { bus, retired, armed: true }
    }
}
impl Drop for RetirementGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            *self.retired = true;
            self.bus.request_shutdown_sync();
        }
    }
}
