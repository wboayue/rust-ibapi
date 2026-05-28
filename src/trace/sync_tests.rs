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
