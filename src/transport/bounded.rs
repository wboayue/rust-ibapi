//! Opt-in, bounded request journals. A local read limit and the native
//! terminal frame are independent: cleanup must still observe the latter
//! after collection stops. No dispatcher waits for a slow reader.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, Weak};

#[cfg(feature = "async")]
use futures::task::AtomicWaker;
#[cfg(feature = "sync")]
use std::sync::Condvar;

use crate::messages::{IncomingMessages, Notice};
use crate::Error;

use super::RoutedItem;

/// Cumulative ingress limits. Consuming a frame does not replenish a budget,
/// so both a stalled reader and a fast reader have bounded retained work.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RawLimits {
    pub frames: usize,
    pub frame_bytes: usize,
    pub total_bytes: usize,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RequestSpec {
    pub data: IncomingMessages,
    pub end: IncomingMessages,
    pub limits: RawLimits,
}

#[derive(Clone, Debug)]
pub(crate) enum Terminal {
    End,
    Error(Error),
}

#[derive(Debug, Default)]
struct Journal {
    queued: VecDeque<RoutedItem>,
    frames: usize,
    bytes: usize,
    read_error: Option<Error>,
    terminal: Option<Terminal>,
    discarding: bool,
}

#[derive(Debug)]
pub(crate) struct Inbox {
    id: i32,
    spec: RequestSpec,
    journal: Mutex<Journal>,
    #[cfg(feature = "async")]
    waker: AtomicWaker,
    #[cfg(feature = "sync")]
    changed: Condvar,
}

impl Inbox {
    fn new(id: i32, spec: RequestSpec) -> Self {
        Self {
            id,
            spec,
            journal: Mutex::new(Journal::default()),
            #[cfg(feature = "async")]
            waker: AtomicWaker::new(),
            #[cfg(feature = "sync")]
            changed: Condvar::new(),
        }
    }

    fn wake(&self) {
        #[cfg(feature = "async")]
        self.waker.wake();
        #[cfg(feature = "sync")]
        self.changed.notify_all();
    }

    /// Enqueue only within the cumulative budget. End state is out of band,
    /// so a full journal cannot evict the only cleanup acknowledgement.
    pub(crate) fn push(&self, item: RoutedItem) {
        let mut journal = self.journal.lock().unwrap();
        if journal.terminal.is_some() {
            return;
        }
        let limits = self.spec.limits;
        match item {
            RoutedItem::Error(error) => {
                // A broker diagnostic can be large too. Do not retain one
                // beyond the per-frame cap, even in the terminal slot.
                let error = match error {
                    Error::Notice(notice) if notice_bytes(&notice) > limits.frame_bytes => limit_error("frame bytes", limits.frame_bytes),
                    error => error,
                };
                journal.terminal = Some(Terminal::Error(error));
            }
            RoutedItem::Response(message) => {
                let size = message.raw_bytes.as_ref().map_or(usize::MAX, Vec::len);
                let is_end = message.message_type() == self.spec.end;
                if message.request_id() != Some(self.id) {
                    journal
                        .read_error
                        .get_or_insert_with(|| Error::UnexpectedResponse("bounded request received a foreign or malformed id".into()));
                } else if size > limits.frame_bytes {
                    journal.read_error.get_or_insert_with(|| limit_error("frame bytes", limits.frame_bytes));
                } else if is_end {
                    // request_id() validated the complete protobuf envelope
                    // with its allocation-free id projection. Native end
                    // frames contain that id and no other required fields.
                    if self.spec.data == self.spec.end {
                        journal.admit(RoutedItem::Response(message), size, limits);
                    }
                    journal.terminal = Some(Terminal::End);
                } else if message.message_type() != self.spec.data {
                    journal
                        .read_error
                        .get_or_insert_with(|| Error::UnexpectedResponse(format!("bounded request received {:?}", message.message_type())));
                } else {
                    journal.admit(RoutedItem::Response(message), size, limits);
                }
            }
            RoutedItem::Notice(notice) => {
                let size = notice_bytes(&notice);
                journal.admit(RoutedItem::Notice(notice), size, limits);
            }
        }
        drop(journal);
        self.wake();
    }

    fn interrupt(&self, error: Error) {
        self.push(RoutedItem::Error(error));
    }
}

fn notice_bytes(notice: &Notice) -> usize {
    notice.message.len().saturating_add(notice.advanced_order_reject_json.len())
}

fn limit_error(resource: &'static str, limit: usize) -> Error {
    Error::ResponseLimitExceeded { resource, limit }
}

impl Journal {
    fn admit(&mut self, item: RoutedItem, size: usize, limits: RawLimits) {
        if self.read_error.is_some() || self.discarding {
            return;
        }
        let exceeded = if size > limits.frame_bytes {
            Some(limit_error("frame bytes", limits.frame_bytes))
        } else if self.frames >= limits.frames {
            Some(limit_error("frames", limits.frames))
        } else if size > limits.total_bytes.saturating_sub(self.bytes) {
            Some(limit_error("total bytes", limits.total_bytes))
        } else {
            None
        };
        if let Some(error) = exceeded {
            self.read_error = Some(error);
        } else {
            self.frames += 1;
            self.bytes += size;
            self.queued.push_back(item);
        }
    }
}

#[derive(Debug, Default)]
struct Registrations {
    entries: HashMap<i32, Weak<Inbox>>,
    closed: bool,
}

/// Separate from the legacy subscription maps: only explicitly bounded
/// queries opt into the journal and its terminal-state semantics.
#[derive(Debug, Default)]
pub(crate) struct Registry {
    registrations: Arc<Mutex<Registrations>>,
}

impl Registry {
    pub(crate) fn register(&self, id: i32, spec: RequestSpec) -> Result<BoundedRead, Error> {
        if id < 0 || spec.limits.frames == 0 || spec.limits.frame_bytes == 0 || spec.limits.total_bytes == 0 {
            return Err(Error::InvalidArgument(
                "bounded request requires a nonnegative id and nonzero limits".into(),
            ));
        }
        let mut registrations = self.registrations.lock()?;
        if registrations.closed {
            return Err(Error::Shutdown);
        }
        if registrations.entries.get(&id).and_then(Weak::upgrade).is_some() {
            return Err(Error::AlreadySubscribed);
        }
        let inbox = Arc::new(Inbox::new(id, spec));
        registrations.entries.insert(id, Arc::downgrade(&inbox));
        Ok(BoundedRead {
            id,
            inbox,
            registrations: Arc::downgrade(&self.registrations),
            read_done: false,
        })
    }

    pub(crate) fn get(&self, id: i32) -> Option<Arc<Inbox>> {
        self.registrations.lock().unwrap().entries.get(&id).and_then(Weak::upgrade)
    }

    /// Take out only the old registrations before waking readers. A retry
    /// registered in response to reset must not be removed by later cleanup.
    pub(crate) fn reset(&self) {
        let old = std::mem::take(&mut self.registrations.lock().unwrap().entries);
        for inbox in old.into_values().filter_map(|entry| entry.upgrade()) {
            inbox.interrupt(Error::ConnectionReset);
        }
    }

    pub(crate) fn close(&self) {
        let old = {
            let mut registrations = self.registrations.lock().unwrap();
            registrations.closed = true;
            std::mem::take(&mut registrations.entries)
        };
        for inbox in old.into_values().filter_map(|entry| entry.upgrade()) {
            inbox.interrupt(Error::Shutdown);
        }
    }
}

impl Drop for Registry {
    fn drop(&mut self) {
        self.close();
    }
}

/// The sole consumer of a journal. Dropping it unregisters synchronously;
/// it does not send a native cancel or prove the request ended remotely.
pub(crate) struct BoundedRead {
    id: i32,
    inbox: Arc<Inbox>,
    registrations: Weak<Mutex<Registrations>>,
    read_done: bool,
}

impl BoundedRead {
    pub(crate) fn terminal(&self) -> Option<Terminal> {
        self.inbox.journal.lock().unwrap().terminal.clone()
    }

    pub(crate) fn discard_buffered(&mut self) {
        let mut journal = self.inbox.journal.lock().unwrap();
        journal.discarding = true;
        journal.queued.clear();
        self.read_done = true;
    }

    fn take_next(&mut self) -> Option<Option<RoutedItem>> {
        if self.read_done {
            return Some(None);
        }
        let mut journal = self.inbox.journal.lock().unwrap();
        if let Some(item) = journal.queued.pop_front() {
            return Some(Some(item));
        }
        if let Some(error) = &journal.read_error {
            self.read_done = true;
            return Some(Some(RoutedItem::Error(error.clone())));
        }
        if let Some(terminal) = &journal.terminal {
            self.read_done = true;
            return Some(match terminal {
                Terminal::End => None,
                Terminal::Error(error) => Some(RoutedItem::Error(error.clone())),
            });
        }
        None
    }

    #[cfg(feature = "async")]
    pub(crate) async fn next_async(&mut self) -> Option<RoutedItem> {
        std::future::poll_fn(|cx| {
            self.inbox.waker.register(cx.waker());
            match self.take_next() {
                Some(item) => std::task::Poll::Ready(item),
                None => std::task::Poll::Pending,
            }
        })
        .await
    }

    #[cfg(feature = "async")]
    pub(crate) async fn terminal_async(&self) -> Terminal {
        std::future::poll_fn(|cx| {
            self.inbox.waker.register(cx.waker());
            match self.terminal() {
                Some(terminal) => std::task::Poll::Ready(terminal),
                None => std::task::Poll::Pending,
            }
        })
        .await
    }

    #[cfg(feature = "sync")]
    pub(crate) fn next_until(&mut self, deadline: std::time::Instant) -> Result<Option<RoutedItem>, Error> {
        loop {
            if let Some(item) = self.take_next() {
                return Ok(item);
            }
            let journal = self.inbox.journal.lock()?;
            if !journal.queued.is_empty() || journal.read_error.is_some() || journal.terminal.is_some() {
                continue;
            }
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(Error::Io(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "bounded request deadline elapsed",
                )));
            }
            drop(self.inbox.changed.wait_timeout(journal, remaining)?);
        }
    }

    #[cfg(feature = "sync")]
    pub(crate) fn terminal_until(&self, deadline: std::time::Instant) -> Result<Terminal, Error> {
        let mut journal = self.inbox.journal.lock()?;
        loop {
            if let Some(terminal) = &journal.terminal {
                return Ok(terminal.clone());
            }
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(Error::Io(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "bounded cleanup deadline elapsed",
                )));
            }
            journal = self.inbox.changed.wait_timeout(journal, remaining)?.0;
        }
    }
}

impl Drop for BoundedRead {
    fn drop(&mut self) {
        if let Some(registrations) = self.registrations.upgrade() {
            let mut registrations = registrations.lock().unwrap();
            if registrations
                .entries
                .get(&self.id)
                .is_some_and(|entry| entry.ptr_eq(&Arc::downgrade(&self.inbox)))
            {
                registrations.entries.remove(&self.id);
            }
        }
    }
}

#[cfg(test)]
#[path = "bounded_tests.rs"]
mod tests;
