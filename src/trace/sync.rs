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
mod tests {
    use super::*;
    use crate::trace::common::storage::sync_ops;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_no_interaction_initially() {
        // Clear to ensure clean initial state
        sync_ops::clear();
        assert!(last_interaction().is_none());
    }

    #[test]
    #[serial]
    fn test_record_and_retrieve_interaction() {
        // Record a request - this replaces any previous interaction
        record_request("TEST_REQUEST".to_string());

        // Should be able to get it back
        let interaction = last_interaction().expect("Should have interaction");
        assert_eq!(interaction.request, "TEST_REQUEST");
        assert_eq!(interaction.responses.len(), 0);
    }

    #[test]
    #[serial]
    fn test_record_request_and_responses() {
        // Record a request - this replaces any previous interaction
        record_request("REQUEST_1".to_string());

        // Record some responses
        record_response("RESPONSE_1".to_string());
        record_response("RESPONSE_2".to_string());

        // Check the interaction
        let interaction = last_interaction().expect("Should have interaction");
        assert_eq!(interaction.request, "REQUEST_1");
        assert_eq!(interaction.responses.len(), 2);
        assert_eq!(interaction.responses[0], "RESPONSE_1");
        assert_eq!(interaction.responses[1], "RESPONSE_2");
    }

    #[test]
    #[serial]
    fn test_new_request_replaces_old() {
        // First interaction
        record_request("REQUEST_1".to_string());
        record_response("RESPONSE_1".to_string());

        // Second interaction - this replaces the first
        record_request("REQUEST_2".to_string());
        record_response("RESPONSE_2".to_string());

        // Should only have the second interaction
        let interaction = last_interaction().expect("Should have interaction");
        assert_eq!(interaction.request, "REQUEST_2");
        assert_eq!(interaction.responses.len(), 1);
        assert_eq!(interaction.responses[0], "RESPONSE_2");
    }
}
