//! Latching shutdown signal shared by the async bus and its connection.

use std::time::Duration;

use tokio::sync::watch;

/// Records that shutdown was requested and wakes anyone waiting on it.
///
/// Unlike `Notify::notify_waiters`, the state latches: a request made while
/// the dispatcher is inside `reconnect()` - with no `Notified` future
/// registered - is still observed by the next wait. `request` takes no lock
/// and needs no runtime, so `Drop` can call it (`request_shutdown_sync`).
#[derive(Debug)]
pub(crate) struct ShutdownSignal {
    sender: watch::Sender<bool>,
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self {
            sender: watch::channel(false).0,
        }
    }
}

impl ShutdownSignal {
    pub(crate) fn is_requested(&self) -> bool {
        *self.sender.borrow()
    }

    pub(crate) fn request(&self) {
        self.sender.send_replace(true);
    }

    /// Resolve once shutdown has been requested, including when it was
    /// requested before this call.
    pub(crate) async fn wait(&self) {
        let mut receiver = self.sender.subscribe();
        while !*receiver.borrow_and_update() {
            if receiver.changed().await.is_err() {
                return;
            }
        }
    }

    /// Sleep for `duration`, returning early once shutdown is requested.
    pub(crate) async fn sleep(&self, duration: Duration) {
        tokio::select! {
            _ = tokio::time::sleep(duration) => {}
            _ = self.wait() => {}
        }
    }
}

#[cfg(test)]
#[path = "shutdown_tests.rs"]
mod tests;
