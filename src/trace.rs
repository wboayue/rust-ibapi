//! Server interaction tracing for debugging and monitoring
//!
//! One process-wide store holds the last interaction: `record_request` starts
//! it and `record_response` appends to it. The functions are plain `fn` under
//! both features, since the lock is held only to replace, append to or clone
//! the interaction. `trace::blocking` names the same functions for code written
//! against `client::blocking`.

use std::sync::RwLock;

/// Represents a single interaction with the server
#[derive(Debug, Clone)]
pub struct Interaction {
    /// The request message that initiated the interaction
    pub request: String,
    /// The response messages received for this request
    pub responses: Vec<String>,
}

impl Interaction {
    /// Creates a new interaction with the given request
    pub(crate) fn new(request: String) -> Self {
        Self {
            request,
            responses: Vec::new(),
        }
    }

    /// Adds a response to this interaction
    pub(crate) fn add_response(&mut self, response: String) {
        self.responses.push(response);
    }
}

/// Global storage for the current interaction
static CURRENT_INTERACTION: RwLock<Option<Interaction>> = RwLock::new(None);

/// Gets the last interaction with the server, if any
///
/// Returns `None` if no interactions have been recorded yet.
///
/// # Example
/// ```no_run
/// use ibapi::trace;
///
/// if let Some(interaction) = trace::last_interaction() {
///     println!("Last request: {}", interaction.request);
///     println!("Responses: {:?}", interaction.responses);
/// }
/// ```
pub fn last_interaction() -> Option<Interaction> {
    CURRENT_INTERACTION.read().ok()?.clone()
}

/// Records a new request, starting a new interaction
///
/// This function starts tracking a new server interaction. Any subsequent
/// calls to `record_response` will add responses to this interaction until
/// a new request is recorded.
///
/// # Arguments
/// * `message` - The request message being sent to the server
///
/// # Example
/// ```no_run
/// use ibapi::trace;
///
/// trace::record_request("REQ|123|AAPL|".to_string());
/// ```
pub fn record_request(message: String) {
    if let Ok(mut guard) = CURRENT_INTERACTION.write() {
        *guard = Some(Interaction::new(message));
    }
}

/// Records a response message for the current interaction
///
/// Adds a response to the most recent interaction started by `record_request`.
/// If no interaction has been started, this function does nothing.
///
/// # Arguments
/// * `message` - The response message received from the server
///
/// # Example
/// ```no_run
/// use ibapi::trace;
///
/// trace::record_request("REQ|123|AAPL|".to_string());
/// trace::record_response("RESP|123|150.00|".to_string());
/// trace::record_response("RESP|123|151.00|".to_string());
/// ```
pub fn record_response(message: String) {
    if let Ok(mut guard) = CURRENT_INTERACTION.write() {
        if let Some(interaction) = guard.as_mut() {
            interaction.add_response(message);
        }
    }
}

/// Clears the current interaction (for testing)
#[cfg(test)]
fn clear() {
    if let Ok(mut guard) = CURRENT_INTERACTION.write() {
        *guard = None;
    }
}

/// The same tracing functions under the path the blocking client uses.
#[cfg(feature = "sync")]
pub mod blocking {
    pub use super::{last_interaction, record_request, record_response};
}

#[cfg(test)]
#[path = "trace_tests.rs"]
mod tests;
