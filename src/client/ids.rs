//! Typed request and order ids, and the partition that keeps them apart.
//!
//! An error frame carries one integer for both domains, so a request and an
//! order holding the same number cannot be told apart on arrival (#789). The
//! `i32` space is split at [`REQUEST_ID_FLOOR`]: the crate mints request ids
//! at or above it, and order ids, which TWS and callers choose, must stay
//! below it. An inbound id's range then names its domain ([`WireId::classify`]).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::Error;

/// First request id. Every request id is at or above it; every order id is
/// below it. Orders get the larger share: their sequence spans the account's
/// lifetime, request ids only one process.
pub(crate) const REQUEST_ID_FLOOR: i32 = 1_500_000_000;

/// Last request id the allocator hands out. TWS drops the id from error
/// frames at `i32::MAX`, which would make an error for that request
/// unroutable.
pub(crate) const REQUEST_ID_CEILING: i32 = i32::MAX - 1;

/// Id TWS sends on error frames that belong to no request or order.
pub(crate) const UNSPECIFIED_REQUEST_ID: i32 = -1;

/// A request id, minted by `ClientIdManager`. Always at or above
/// [`REQUEST_ID_FLOOR`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub(crate) struct RequestId(i32);

impl RequestId {
    /// `id` as a request id, or `None` below the floor.
    pub(crate) fn from_raw(id: i32) -> Option<Self> {
        (id >= REQUEST_ID_FLOOR).then_some(Self(id))
    }

    pub(crate) fn raw(self) -> i32 {
        self.0
    }
}

#[cfg(test)]
impl RequestId {
    /// The `n`th request id from the floor.
    pub(crate) const fn nth(n: i32) -> Self {
        Self(REQUEST_ID_FLOOR + n)
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// An order identifier.
///
/// Converts from any `i32`. Order ids must stay below 1,500,000,000, the
/// range reserved for request ids: the order methods return
/// [`Error::OrderIdInRequestRange`] for one at or above it.
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderId(pub i32);

impl OrderId {
    /// Creates a new OrderId
    pub fn new(id: i32) -> Self {
        Self(id)
    }

    /// Returns the inner i32 value
    pub fn value(&self) -> i32 {
        self.0
    }

    /// This id, or [`Error::OrderIdInRequestRange`] if it falls in the
    /// request range, where its error frames would route as a request's.
    pub(crate) fn checked(self) -> Result<Self, Error> {
        if self.0 >= REQUEST_ID_FLOOR {
            Err(Error::OrderIdInRequestRange { order_id: self.0 })
        } else {
            Ok(self)
        }
    }
}

impl fmt::Display for OrderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<i32> for OrderId {
    fn from(id: i32) -> Self {
        Self(id)
    }
}

impl From<OrderId> for i32 {
    fn from(id: OrderId) -> i32 {
        id.0
    }
}

/// The domain of an id read off the wire, decided by range alone.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum WireId {
    Request(RequestId),
    Order(OrderId),
}

impl fmt::Display for WireId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request(id) => id.fmt(f),
            Self::Order(id) => id.fmt(f),
        }
    }
}

impl WireId {
    /// `None` for TWS's unspecified id (`-1`). Every other value below the
    /// floor is an order id, negatives included: orders placed in TWS itself
    /// carry negative ids.
    pub(crate) fn classify(id: i32) -> Option<Self> {
        if id == UNSPECIFIED_REQUEST_ID {
            None
        } else if let Some(request_id) = RequestId::from_raw(id) {
            Some(Self::Request(request_id))
        } else {
            Some(Self::Order(OrderId(id)))
        }
    }
}

#[cfg(test)]
#[path = "ids_tests.rs"]
mod tests;
