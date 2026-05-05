//! Connection-status monitor using `Client::notice_stream()`.
//!
//! Subscribes to globally routed IB notices — codes that arrive without a
//! `request_id` — and prints a categorized line for each one. Typical traffic
//! includes connectivity changes (1100/1101/1102) and farm-status updates
//! (2104/2105/2106/2107/2108).
//!
//! ```bash
//! cargo run --features sync --example notice_stream
//! ```

use ibapi::client::blocking::Client;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100)?;
    println!("connected; server version {}", client.server_version());

    let stream = client.notice_stream()?;
    println!("listening for global notices (Ctrl+C to exit)...");

    for notice in stream.iter() {
        if notice.is_system_message() {
            println!("[connectivity] {notice}");
        } else if notice.is_warning() {
            println!("[warning]      {notice}");
        } else {
            eprintln!("[error]        {notice}");
        }
    }
    Ok(())
}
