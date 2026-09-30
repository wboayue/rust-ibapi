//! Opt-in bounded, single-attempt reference-data requests.
//!
//! Preparation exposes ownership and the request ID before any write. Limits
//! bound retained raw frames and decoded work; they do not bound Gateway work
//! already requested. Legacy collecting APIs are unchanged. No retry or pacing
//! is hidden here: callers own admission and the total operation deadline.
//!
//! An end marker proves remote completion, not that every row was collected.
//! In particular a local limit can stop reading before the native end arrives.
//! Use the read result and [`QueryDisposition`] together.

use crate::messages::{IncomingMessages, ResponseMessage};
use crate::subscriptions::SubscriptionItem;
use crate::transport::bounded::{BoundedRead, RawLimits, RequestSpec, Terminal};
use crate::transport::RoutedItem;
use crate::Error;

#[path = "enumeration_decode.rs"]
mod decode;

#[cfg(feature = "async")]
#[path = "enumeration_async.rs"]
pub mod async_impl;
#[cfg(feature = "sync")]
#[path = "enumeration_sync.rs"]
pub mod sync_impl;

#[cfg(feature = "async")]
pub use async_impl::ContractQuery;
#[cfg(all(feature = "sync", not(feature = "async")))]
pub use sync_impl::ContractQuery;

/// Cumulative local budgets for one request, validated during preparation.
/// All values must be nonzero. Consumption never replenishes a budget.
///
/// The transport has its own global frame-size ceiling before request routing;
/// `frame_bytes` is a tighter per-request retention/decode limit, not a promise
/// that the socket never temporarily allocates a larger frame.
///
/// # Examples
///
/// ```
/// use ibapi::contracts::QueryLimits;
/// let limits = QueryLimits { rows: 32, ..QueryLimits::default() };
/// assert_eq!(limits.rows, 32);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueryLimits {
    /// Native result rows, before any consumer filtering. A symbol-samples
    /// response counts each description, not just its single enclosing frame.
    pub rows: usize,
    /// Raw data/notice frames, including notices a caller chooses to ignore.
    pub frames: usize,
    /// Bytes retained in any individual data frame or broker diagnostic.
    pub frame_bytes: usize,
    /// Cumulative bytes of queued data and notices, even after consumption.
    pub total_bytes: usize,
    /// Protobuf fields plus packed array elements across decoded responses.
    /// Nested contract fields, map entries, and duplicate fields count too.
    /// Checked without allocating domain arrays or maps.
    pub decode_entries: usize,
}

impl Default for QueryLimits {
    fn default() -> Self {
        Self {
            rows: 256,
            frames: 4096,
            frame_bytes: 64 * 1024,
            total_bytes: 2 * 1024 * 1024,
            decode_entries: 8192,
        }
    }
}

impl QueryLimits {
    fn validate(self) -> Result<Self, Error> {
        if [self.rows, self.frames, self.frame_bytes, self.total_bytes, self.decode_entries].contains(&0) {
            return Err(Error::InvalidArgument("query limits must all be nonzero".into()));
        }
        Ok(self)
    }

    fn raw(self) -> RawLimits {
        RawLimits {
            frames: self.frames,
            frame_bytes: self.frame_bytes,
            total_bytes: self.total_bytes,
        }
    }
}

/// Request-lifecycle evidence, **not** a completeness flag for collected rows.
/// Even `ResponseEnded` may follow a local row/decode limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryDisposition {
    /// No submission was attempted; dropping the handle needs no retirement.
    NotSubmitted,
    /// Written once; no native terminal response has been observed yet.
    Pending,
    /// A valid, request-owned native end was received (symbol samples are
    /// themselves the single-frame end of that request).
    ResponseEnded,
    /// Native code 200 rejected the definition request. The original notice
    /// is returned by the read path; rejection alone does not retire a session.
    DefinitionRejected,
    /// Uncertain write, interruption, unconfirmed cleanup or explicit
    /// retirement. This handle must not certify reuse of the client session.
    RetireRequired,
}

pub(crate) struct QueryPlan<T> {
    id: i32,
    packet: Vec<u8>,
    spec: RequestSpec,
    decoder: fn(&ResponseMessage, &mut decode::Budget) -> Result<T, Error>,
    limits: QueryLimits,
}

impl<T> QueryPlan<T> {
    fn new(id: i32, packet: Vec<u8>, format: Format<T>) -> Self {
        Self {
            id,
            packet,
            spec: format.spec,
            decoder: format.decoder,
            limits: format.limits,
        }
    }

    pub(super) fn core(self, read: BoundedRead) -> QueryCore<T> {
        QueryCore {
            plan: self,
            read,
            attempted: false,
            uncertain: false,
            retired: false,
            read_stopped: false,
            cancel_attempted: false,
            budget: decode::Budget::default(),
        }
    }
}

struct Format<T> {
    spec: RequestSpec,
    decoder: fn(&ResponseMessage, &mut decode::Budget) -> Result<T, Error>,
    limits: QueryLimits,
}

impl<T> Format<T> {
    fn new(
        limits: QueryLimits,
        messages: (IncomingMessages, IncomingMessages),
        decoder: fn(&ResponseMessage, &mut decode::Budget) -> Result<T, Error>,
    ) -> Result<Self, Error> {
        let limits = limits.validate()?;
        Ok(Self {
            spec: RequestSpec {
                data: messages.0,
                end: messages.1,
                limits: limits.raw(),
            },
            decoder,
            limits,
        })
    }
}

pub(super) struct QueryCore<T> {
    plan: QueryPlan<T>,
    read: BoundedRead,
    attempted: bool,
    uncertain: bool,
    retired: bool,
    read_stopped: bool,
    cancel_attempted: bool,
    budget: decode::Budget,
}

impl<T> QueryCore<T> {
    fn disposition(&self) -> QueryDisposition {
        if self.retired || self.uncertain {
            return QueryDisposition::RetireRequired;
        }
        if !self.attempted {
            return QueryDisposition::NotSubmitted;
        }
        match self.read.terminal() {
            Some(terminal) => terminal_disposition(terminal).unwrap_or(QueryDisposition::RetireRequired),
            None => QueryDisposition::Pending,
        }
    }

    fn begin(&mut self) -> Result<(), Error> {
        if self.attempted {
            return Err(Error::InvalidArgument("query can only be submitted once".into()));
        }
        if let Some(terminal) = self.read.terminal() {
            return Err(match terminal {
                Terminal::Error(error) => error,
                Terminal::End => Error::UnexpectedResponse("response arrived before submission".into()),
            });
        }
        self.attempted = true;
        self.uncertain = true;
        self.budget = decode::Budget::new(self.plan.limits);
        Ok(())
    }

    /// Undo `begin` after the send gate refused before any byte was written:
    /// nothing with this id is on the wire, so the query is not submitted.
    fn unsubmit(&mut self) {
        self.attempted = false;
        self.uncertain = false;
    }

    fn before_read(&self) -> Result<bool, Error> {
        if !self.attempted {
            return Err(Error::InvalidArgument("start the prepared query before reading".into()));
        }
        // Retirement prevents reuse/writes, not access to already-confirmed
        // input. Shutdown closes the journal with its queued prefix intact;
        // expose that prefix, then its terminal error, then fuse normally.
        Ok(!self.read_stopped)
    }

    fn accept(&mut self, item: Option<RoutedItem>) -> Result<Option<SubscriptionItem<T>>, Error> {
        let result = match item {
            Some(RoutedItem::Response(message)) => (self.plan.decoder)(&message, &mut self.budget).map(|value| Some(SubscriptionItem::Data(value))),
            Some(RoutedItem::Notice(notice)) => Ok(Some(SubscriptionItem::Notice(notice))),
            Some(RoutedItem::Error(error)) => Err(error),
            None => Ok(None),
        };
        if !matches!(result, Ok(Some(_))) {
            self.read_stopped = true;
        }
        result
    }

    fn cancel_packet(&self, version: i32) -> Result<Option<Vec<u8>>, Error> {
        if self.plan.spec.data == IncomingMessages::ContractData && version >= crate::server_versions::CANCEL_CONTRACT_DATA {
            super::common::encoders::encode_cancel_contract_data(self.plan.id).map(Some)
        } else {
            Ok(None)
        }
    }
}

fn terminal_disposition(terminal: Terminal) -> Result<QueryDisposition, Error> {
    match terminal {
        Terminal::End => Ok(QueryDisposition::ResponseEnded),
        Terminal::Error(Error::Notice(notice)) if notice.code == 200 => Ok(QueryDisposition::DefinitionRejected),
        Terminal::Error(error) => Err(error),
    }
}

pub(super) fn details_plan(id: i32, contract: &super::Contract, limits: QueryLimits) -> Result<QueryPlan<super::ContractDetails>, Error> {
    let format = Format::new(
        limits,
        (IncomingMessages::ContractData, IncomingMessages::ContractDataEnd),
        decode::details,
    )?;
    Ok(QueryPlan::new(
        id,
        super::common::encoders::encode_request_contract_data(id, contract)?,
        format,
    ))
}

pub(super) fn symbols_plan(id: i32, pattern: &str, limits: QueryLimits) -> Result<QueryPlan<Vec<super::ContractDescription>>, Error> {
    let format = Format::new(
        limits,
        (IncomingMessages::SymbolSamples, IncomingMessages::SymbolSamples),
        decode::symbols,
    )?;
    Ok(QueryPlan::new(
        id,
        super::common::encoders::encode_request_matching_symbols(id, pattern)?,
        format,
    ))
}

pub(super) fn chain_plan(id: i32, packet: Vec<u8>, limits: QueryLimits) -> Result<QueryPlan<super::OptionChain>, Error> {
    let format = Format::new(
        limits,
        (
            IncomingMessages::SecurityDefinitionOptionParameter,
            IncomingMessages::SecurityDefinitionOptionParameterEnd,
        ),
        decode::chain,
    )?;
    Ok(QueryPlan::new(id, packet, format))
}
