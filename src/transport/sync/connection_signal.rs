//! Connection-state signal shared by the blocking bus and the callers waiting
//! out a reconnect.

use std::sync::{Condvar, Mutex, PoisonError};

use crate::errors::Error;
use crate::transport::common::{lock, SessionState};

/// Whether the session is live, and a way to block until it is again.
///
/// The dispatcher moves it to `Disconnected` on a connection-lost read and
/// back to `Connected` once the replayed handshake completes, or to `Failed`
/// when the reconnect gives up or the dispatcher dies on a read error.
/// `ShutDown` and `Failed` latch: they are terminal, so a waiter is never left
/// blocked on a session that will not return.
///
/// Same shape as [`ShutdownSignal`](super::ShutdownSignal) - `Mutex` +
/// `Condvar`, no polling.
#[derive(Debug)]
pub(crate) struct ConnectionSignal {
    state: Mutex<SessionState>,
    changed: Condvar,
}

impl Default for ConnectionSignal {
    fn default() -> Self {
        Self {
            state: Mutex::new(SessionState::Connected),
            changed: Condvar::new(),
        }
    }
}

impl ConnectionSignal {
    pub(crate) fn is_connected(&self) -> bool {
        *lock(&self.state) == SessionState::Connected
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
        lock(&self.state).check_ended()
    }

    /// `Ok` while connected; otherwise [`Error::ConnectionReset`] while a
    /// reconnect may still succeed, [`Error::ConnectionFailed`] once it gave
    /// up, [`Error::Shutdown`] after a shutdown.
    pub(crate) fn check(&self) -> Result<(), Error> {
        lock(&self.state).check()
    }

    /// Block until the session is connected again, returning
    /// [`Error::ConnectionFailed`] or [`Error::Shutdown`] if it never will be.
    pub(crate) fn wait_connected(&self) -> Result<(), Error> {
        let mut state = lock(&self.state);
        while *state == SessionState::Disconnected {
            state = self.changed.wait(state).unwrap_or_else(PoisonError::into_inner);
        }
        state.check()
    }

    fn set(&self, new_state: SessionState) {
        let mut state = lock(&self.state);
        // Terminal states latch: a reconnect that lands after one must not
        // report the session live again, and a shutdown after a failed
        // reconnect must not hide why the session ended.
        if state.is_terminal() {
            return;
        }
        *state = new_state;
        self.changed.notify_all();
    }
}

#[cfg(test)]
#[path = "connection_signal_tests.rs"]
mod tests;
