use super::*;
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
