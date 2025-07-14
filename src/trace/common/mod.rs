pub(super) mod storage;

use std::sync::Arc;

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

/// Type alias for shared interaction storage
pub(super) type SharedInteraction = Arc<Interaction>;
