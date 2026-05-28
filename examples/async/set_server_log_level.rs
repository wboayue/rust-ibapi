//! Set Server Log Level example (async).
//!
//! Adjusts the verbosity of server-side TWS API diagnostics.
//!
//! ```bash
//! cargo run --example async_set_server_log_level -- detail
//! ```
//!
//! Accepted levels: `system`, `error`, `warning`, `info`, `detail`.

use ibapi::accounts::ServerLogLevel;
use ibapi::prelude::*;

#[tokio::main]
async fn main() {
    env_logger::init();

    let arg = std::env::args().nth(1).unwrap_or_else(|| "detail".to_string());
    let level = match arg.as_str() {
        "system" => ServerLogLevel::System,
        "error" => ServerLogLevel::Error,
        "warning" => ServerLogLevel::Warning,
        "info" => ServerLogLevel::Info,
        "detail" => ServerLogLevel::Detail,
        other => {
            eprintln!("unknown log level: {other}. expected system|error|warning|info|detail");
            std::process::exit(2);
        }
    };

    let client = Client::connect("127.0.0.1:4002", 100).await.expect("connection failed");

    client.set_server_log_level(level).await.expect("set_server_log_level failed");

    println!("server log level set to {level:?}");
}
