//! The answer to a what-if order, for the sync and async `analyze`.

use std::time::{Duration, Instant};

use crate::common::request_helpers::ONE_SHOT_TIMEOUT;
use crate::orders::OrderState;

/// Assembles the answer to a what-if order from the open-order frames TWS replies with.
///
/// Live, server 223: when TWS has a precautionary warning for the order (a US stock limit
/// order), it answers with two frames. The first carries only the status, a commission of
/// `0` and `warning_text`; the margins and commission follow in a second frame without the
/// warning. When the account cannot carry the order, the frame with margins has
/// `suggested_size` set and no commission, and a 201 rejection follows it.
#[derive(Debug, Default)]
pub(crate) struct WhatIfReply {
    warning_text: String,
}

impl WhatIfReply {
    /// The answer, once `state` carries it; `None` to keep reading. The answer keeps the
    /// warning of an earlier warning-only frame.
    pub(crate) fn observe(&mut self, mut state: OrderState) -> Option<OrderState> {
        if state.initial_margin_before.is_none() && !state.warning_text.is_empty() {
            self.warning_text = state.warning_text;
            return None;
        }
        if state.suggested_size.is_some() {
            return None;
        }
        if state.warning_text.is_empty() {
            state.warning_text = std::mem::take(&mut self.warning_text);
        }
        Some(state)
    }
}

pub(crate) fn reply_deadline(start: Instant, timeout: Option<Duration>) -> Option<Instant> {
    start.checked_add(timeout.unwrap_or(ONE_SHOT_TIMEOUT))
}

#[cfg(test)]
#[path = "what_if_tests.rs"]
mod tests;
