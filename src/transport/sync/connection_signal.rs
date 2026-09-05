//! Connection-state signal shared by the blocking bus and the callers waiting
//! out a reconnect.

use std::sync::{Condvar, Mutex, PoisonError};

use crate::errors::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Connected,
    Disconnected,
    ShutDown,
}

/// Whether the session is live, and a way to block until it is again.
///
/// The dispatcher moves it to `Disconnected` on a connection-lost read and
/// back to `Connected` once the replayed handshake completes. `ShutDown`
/// latches: it is terminal, so a waiter is never left blocked on a session
/// that will not return.
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
        *self.state.lock().unwrap_or_else(PoisonError::into_inner) == State::Connected
    }

    pub(crate) fn set_connected(&self) {
        self.set(State::Connected);
    }

    pub(crate) fn set_disconnected(&self) {
        self.set(State::Disconnected);
    }

    /// Latch the terminal state and wake every waiter.
    pub(crate) fn shutdown(&self) {
        *self.state.lock().unwrap_or_else(PoisonError::into_inner) = State::ShutDown;
        self.changed.notify_all();
    }

    /// Block until the session is connected again, returning
    /// [`Error::Shutdown`] if it never will be.
    pub(crate) fn wait_connected(&self) -> Result<(), Error> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        loop {
            match *state {
                State::Connected => return Ok(()),
                State::ShutDown => return Err(Error::Shutdown),
                State::Disconnected => state = self.changed.wait(state).unwrap_or_else(PoisonError::into_inner),
            }
        }
    }

    fn set(&self, new_state: State) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        // Shutdown is terminal: a reconnect that lands after it must not
        // report the session live again.
        if *state == State::ShutDown {
            return;
        }
        *state = new_state;
        self.changed.notify_all();
    }
}

#[cfg(test)]
#[path = "connection_signal_tests.rs"]
mod tests;
