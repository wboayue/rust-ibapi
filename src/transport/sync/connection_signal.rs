//! Connection-state signal shared by the blocking bus and the callers waiting
//! out a reconnect.

use std::sync::{Condvar, Mutex, PoisonError};

use crate::errors::Error;
use crate::transport::common::lock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Connected,
    Disconnected,
    ShutDown,
    Failed,
}

impl State {
    /// `ShutDown` and `Failed` latch: neither a reconnect nor a later
    /// shutdown moves the session out of them.
    fn is_terminal(self) -> bool {
        matches!(self, State::ShutDown | State::Failed)
    }

    /// The gate's verdict: live, reconnecting, or gone for good.
    fn check(self) -> Result<(), Error> {
        match self {
            State::Connected => Ok(()),
            State::Disconnected => Err(Error::ConnectionReset),
            State::ShutDown => Err(Error::Shutdown),
            State::Failed => Err(Error::ConnectionFailed),
        }
    }
}

/// Whether the session is live, and a way to block until it is again.
///
/// The dispatcher moves it to `Disconnected` on a connection-lost read and
/// back to `Connected` once the replayed handshake completes, or to `Failed`
/// when the reconnect gives up. `ShutDown` and `Failed` latch: they are
/// terminal, so a waiter is never left blocked on a session that will not
/// return.
///
/// Same shape as [`ShutdownSignal`](super::ShutdownSignal) - `Mutex` +
/// `Condvar`, no polling.
#[derive(Debug)]
pub(crate) struct ConnectionSignal {
    state: Mutex<State>,
    changed: Condvar,
}

impl Default for ConnectionSignal {
    fn default() -> Self {
        Self {
            state: Mutex::new(State::Connected),
            changed: Condvar::new(),
        }
    }
}

impl ConnectionSignal {
    pub(crate) fn is_connected(&self) -> bool {
        *lock(&self.state) == State::Connected
    }

    pub(crate) fn set_connected(&self) {
        self.set(State::Connected);
    }

    pub(crate) fn set_disconnected(&self) {
        self.set(State::Disconnected);
    }

    /// Latch the shutdown state and wake every waiter.
    pub(crate) fn shutdown(&self) {
        self.set(State::ShutDown);
    }

    /// Latch the reconnect-exhausted state and wake every waiter. Call before
    /// the shutdown that follows, which then keeps it.
    pub(crate) fn fail(&self) {
        self.set(State::Failed);
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
        while *state == State::Disconnected {
            state = self.changed.wait(state).unwrap_or_else(PoisonError::into_inner);
        }
        state.check()
    }

    fn set(&self, new_state: State) {
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
