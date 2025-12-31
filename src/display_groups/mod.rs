//! # Display Groups
//!
//! This module provides functionality for subscribing to TWS Display Group events.
//! Display Groups in TWS allow users to organize contracts into color-coded groups
//! (e.g., Group 1 = Red, Group 2 = Orange, etc.).
//!
//! When subscribed to a display group, you receive updates whenever the user
//! changes the contract displayed in that group within TWS.

pub(crate) mod common;

#[cfg(feature = "async")]
pub(crate) mod r#async;

#[cfg(feature = "sync")]
pub(crate) mod sync;

pub(crate) use common::encoders;

pub use common::DisplayGroupUpdate;
