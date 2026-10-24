//! Connection-state signal shared by the async bus and the callers waiting out
//! a reconnect.

use tokio::sync::watch;

use crate::errors::Error;
use crate::transport::common::SessionState;

/// Whether the session is live, and a way to await its return.
///
/// The dispatcher moves it to `Disconnected` on a connection-lost read and
/// back to `Connected` once the replayed handshake completes, or to `Failed`
/// when the reconnect gives up or the dispatcher dies on a read error.
/// `ShutDown` and `Failed` latch: they are terminal, so a waiter is never left
/// awaiting a session that will not return.
///
/// Same shape as [`ShutdownSignal`](super::ShutdownSignal) - a `watch`
/// channel, so `Drop` can move it to `ShutDown` without a runtime.
#[derive(Debug)]
pub(crate) struct ConnectionSignal {
    sender: watch::Sender<SessionState>,
}

impl Default for ConnectionSignal {
    fn default() -> Self {
        Self {
            sender: watch::channel(SessionState::Connected).0,
        }
    }
}

impl ConnectionSignal {
    pub(crate) fn is_connected(&self) -> bool {
        *self.sender.borrow() == SessionState::Connected
    }

    pub(crate) fn set_connected(&self) {
        self.set(SessionState::Connected);
    }

    pub(crate) fn set_disconnected(&self) {
        self.set(SessionState::Disconnected);
    }

    /// Latch the shutdown state and wake every waiter.
    pub(crate) fn shutdown(&self) {
        self.set(SessionState::ShutDown);
    }

    /// Latch the failed state (the reconnect gave up, or the dispatcher died)
    /// and wake every waiter. Call before the shutdown that follows, which
    /// then keeps it.
    pub(crate) fn fail(&self) {
        self.set(SessionState::Failed);
    }

    /// `Ok` unless the session ended for good: [`Error::ConnectionFailed`]
    /// or [`Error::Shutdown`], as [`check`](Self::check) reports them.
    pub(crate) fn check_ended(&self) -> Result<(), Error> {
        self.sender.borrow().check_ended()
    }

    /// `Ok` while connected; otherwise [`Error::ConnectionReset`] while a
    /// reconnect may still succeed, [`Error::ConnectionFailed`] once it gave
    /// up, [`Error::Shutdown`] after a shutdown.
    pub(crate) fn check(&self) -> Result<(), Error> {
        self.sender.borrow().check()
    }

    /// Resolve once the session is connected again, returning
    /// [`Error::ConnectionFailed`] or [`Error::Shutdown`] if it never will be.
    pub(crate) async fn wait_connected(&self) -> Result<(), Error> {
        let mut receiver = self.sender.subscribe();
        loop {
            let state = *receiver.borrow_and_update();
            if state != SessionState::Disconnected {
                return state.check();
            }
            // The bus outlives every waiter, but a closed channel still
            // means no reconnect is coming.
            if receiver.changed().await.is_err() {
                return Err(Error::Shutdown);
            }
        }
    }

    fn set(&self, new_state: SessionState) {
        // Terminal states latch: a reconnect that lands after one must not
        // report the session live again, and a shutdown after a failed
        // reconnect must not hide why the session ended.
        self.sender.send_if_modified(|state| {
            if state.is_terminal() || *state == new_state {
                return false;
            }
            *state = new_state;
            true
        });
    }
}

#[cfg(test)]
#[path = "connection_signal_tests.rs"]
mod tests;
