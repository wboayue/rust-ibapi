//! Asynchronous client implementation

use crate::Error;

/// Asynchronous TWS API Client
pub struct Client {
    // TODO: Implement async client
}

impl Client {
    /// Establishes async connection to TWS or Gateway
    pub async fn connect(_address: &str, _client_id: i32) -> Result<Client, Error> {
        todo!("Implement async client")
    }
}
