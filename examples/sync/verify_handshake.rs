//! IB Linking verification handshake example (sync).
//!
//! Runs both halves of the verification handshake: `verify_request` to
//! receive an API challenge, then `verify_message` with the signed
//! response.
//!
//! Most users will not need to call this directly — it's part of the IB
//! Linking extension authentication flow.
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example verify_handshake -- MyApp 1.0 signed-data
//! ```

use ibapi::client::blocking::Client;

fn main() {
    env_logger::init();

    let api_name = std::env::args().nth(1).unwrap_or_else(|| "MyApp".to_string());
    let api_version = std::env::args().nth(2).unwrap_or_else(|| "1.0".to_string());
    let signed_response = std::env::args().nth(3).unwrap_or_else(|| "signed-data".to_string());

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let challenge = client.verify_request(&api_name, &api_version).expect("verify_request failed");
    println!("challenge: {}", challenge.api_data);

    let result = client.verify_message(&signed_response).expect("verify_message failed");
    if result.is_successful {
        println!("verification succeeded");
    } else {
        println!("verification failed: {}", result.error_text);
    }
}
