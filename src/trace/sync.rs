use crate::trace::common::{storage::sync_ops, Interaction};

/// Gets the last interaction with the server, if any
///
/// Returns `None` if no interactions have been recorded yet.
///
/// # Example
/// ```no_run
/// use ibapi::trace::blocking;
///
/// if let Some(interaction) = blocking::last_interaction() {
///     println!("Last request: {}", interaction.request);
///     println!("Responses: {:?}", interaction.responses);
/// }
/// ```
pub fn last_interaction() -> Option<Interaction> {
    sync_ops::get_last_interaction().map(|arc| (*arc).clone())
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
/// use ibapi::trace::blocking;
///
/// blocking::record_request("REQ|123|AAPL|".to_string());
/// ```
pub fn record_request(message: String) {
    sync_ops::start_new_interaction(message);
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
/// use ibapi::trace::blocking;
///
/// blocking::record_request("REQ|123|AAPL|".to_string());
/// blocking::record_response("RESP|123|150.00|".to_string());
/// blocking::record_response("RESP|123|151.00|".to_string());
/// ```
pub fn record_response(message: String) {
    sync_ops::add_response_to_current(message);
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
