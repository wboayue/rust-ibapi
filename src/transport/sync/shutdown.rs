//! Latching shutdown signal shared by the blocking bus and its connection.

use std::sync::{Condvar, Mutex, PoisonError};
use std::time::Duration;

/// Records that shutdown was requested and wakes anyone waiting on it.
///
/// The flag latches: a request made before a wait starts is still observed.
/// `Connection::reconnect` holds one so its backoff wait ends as soon as the
/// bus (or `Client::drop`) asks to shut down, instead of running the whole
/// Fibonacci schedule.
#[derive(Debug, Default)]
pub(crate) struct ShutdownSignal {
    requested: Mutex<bool>,
    changed: Condvar,
}

impl ShutdownSignal {
    pub(crate) fn is_requested(&self) -> bool {
        *self.requested.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn request(&self) {
        *self.requested.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.changed.notify_all();
    }

    /// Block for up to `duration`, returning early once shutdown is requested.
    pub(crate) fn wait_timeout(&self, duration: Duration) {
        let requested = self.requested.lock().unwrap_or_else(PoisonError::into_inner);
        let _ = self
            .changed
            .wait_timeout_while(requested, duration, |requested| !*requested)
            .map_err(PoisonError::into_inner);
    }
}

#[cfg(test)]
#[path = "shutdown_tests.rs"]
mod tests;
