//! Connection-status monitor using `Client::notice_stream()` (async).
//!
//! Subscribes to globally routed IB notices — codes that arrive without a
//! `request_id` — and prints a categorized line for each one. Typical traffic
//! includes connectivity changes (1100/1101/1102) and farm-status updates
//! (2104/2105/2106/2107/2108).
//!
//! ```bash
//! cargo run --features async --example async_notice_stream
//! ```

use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).await?;
    println!("connected; server version {}", client.server_version());

    let mut stream = client.notice_stream()?;
    println!("listening for global notices (Ctrl+C to exit)...");

    while let Some(notice) = stream.next().await {
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
