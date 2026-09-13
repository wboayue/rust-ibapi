//! Connection-state signal shared by the async bus and the callers waiting out
//! a reconnect.

use tokio::sync::watch;

use crate::errors::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Connected,
    Disconnected,
    ShutDown,
}

/// Whether the session is live, and a way to await its return.
///
/// The dispatcher moves it to `Disconnected` on a connection-lost read and
/// back to `Connected` once the replayed handshake completes. `ShutDown`
/// latches: it is terminal, so a waiter is never left awaiting a session that
/// will not return.
///
/// Same shape as [`ShutdownSignal`](super::ShutdownSignal) - a `watch`
/// channel, so `Drop` can move it to `ShutDown` without a runtime.
#[derive(Debug)]
pub(crate) struct ConnectionSignal {
    sender: watch::Sender<State>,
}

impl Default for ConnectionSignal {
    fn default() -> Self {
        Self {
            sender: watch::channel(State::Connected).0,
        }
    }
}

impl ConnectionSignal {
    pub(crate) fn is_connected(&self) -> bool {
        *self.sender.borrow() == State::Connected
    }

    pub(crate) fn set_connected(&self) {
        self.set(State::Connected);
    }

    pub(crate) fn set_disconnected(&self) {
        self.set(State::Disconnected);
    }

    /// Latch the terminal state and wake every waiter.
    pub(crate) fn shutdown(&self) {
        self.sender.send_replace(State::ShutDown);
    }

    /// Resolve once the session is connected again, returning
    /// [`Error::Shutdown`] if it never will be.
    pub(crate) async fn wait_connected(&self) -> Result<(), Error> {
        let mut receiver = self.sender.subscribe();
        loop {
            let state = *receiver.borrow_and_update();
            match state {
                State::Connected => return Ok(()),
                State::ShutDown => return Err(Error::Shutdown),
                // The bus outlives every waiter, but a closed channel still
                // means no reconnect is coming.
                State::Disconnected if receiver.changed().await.is_err() => return Err(Error::Shutdown),
                State::Disconnected => {}
            }
        }
    }

    fn set(&self, new_state: State) {
        // Shutdown is terminal: a reconnect that lands after it must not
        // report the session live again.
        self.sender.send_if_modified(|state| {
            if *state == State::ShutDown || *state == new_state {
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
