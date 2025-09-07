use crate::trace::common::{storage::async_ops, Interaction};

/// Gets the last interaction with the server, if any
///
/// Returns `None` if no interactions have been recorded yet.
///
/// # Example
/// ```no_run
/// use ibapi::trace;
///
/// # async fn example() {
/// if let Some(interaction) = trace::last_interaction().await {
///     println!("Last request: {}", interaction.request);
///     println!("Responses: {:?}", interaction.responses);
/// }
/// # }
/// ```
pub async fn last_interaction() -> Option<Interaction> {
    async_ops::get_last_interaction().await.map(|arc| (*arc).clone())
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
/// # async fn example() {
/// trace::record_request("REQ|123|AAPL|".to_string()).await;
/// # }
/// ```
pub async fn record_request(message: String) {
    async_ops::start_new_interaction(message).await;
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
/// # async fn example() {
/// trace::record_request("REQ|123|AAPL|".to_string()).await;
/// trace::record_response("RESP|123|150.00|".to_string()).await;
/// trace::record_response("RESP|123|151.00|".to_string()).await;
/// # }
/// ```
pub async fn record_response(message: String) {
    async_ops::add_response_to_current(message).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{Currency, Exchange, Symbol};
    use crate::trace::common::storage::async_ops;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_no_interaction_initially() {
        // Clear to ensure clean initial state
        async_ops::clear().await;
        assert!(last_interaction().await.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_record_and_retrieve_interaction() {
        // Record a request - this replaces any previous interaction
        record_request("TEST_REQUEST".to_string()).await;

        // Should be able to get it back
        let interaction = last_interaction().await.expect("Should have interaction");
        assert_eq!(interaction.request, "TEST_REQUEST");
        assert_eq!(interaction.responses.len(), 0);
    }

    #[tokio::test]
    #[serial]
    async fn test_record_request_and_responses() {
        // Record a request - this replaces any previous interaction
        record_request("REQUEST_1".to_string()).await;

        // Record some responses
        record_response("RESPONSE_1".to_string()).await;
        record_response("RESPONSE_2".to_string()).await;

        // Check the interaction
        let interaction = last_interaction().await.expect("Should have interaction");
        assert_eq!(interaction.request, "REQUEST_1");
        assert_eq!(interaction.responses.len(), 2);
        assert_eq!(interaction.responses[0], "RESPONSE_1");
        assert_eq!(interaction.responses[1], "RESPONSE_2");
    }

    #[tokio::test]
    #[serial]
    async fn test_new_request_replaces_old() {
        // First interaction
        record_request("REQUEST_1".to_string()).await;
        record_response("RESPONSE_1".to_string()).await;

        // Second interaction - this replaces the first
        record_request("REQUEST_2".to_string()).await;
        record_response("RESPONSE_2".to_string()).await;

        // Should only have the second interaction
        let interaction = last_interaction().await.expect("Should have interaction");
        assert_eq!(interaction.request, "REQUEST_2");
        assert_eq!(interaction.responses.len(), 1);
        assert_eq!(interaction.responses[0], "RESPONSE_2");
    }
}
