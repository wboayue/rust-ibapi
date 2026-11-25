//! Connect to TWS or IB Gateway from a WASI (wasm32-wasip2) component.
//!
//! On wasm the async client has no built-in TCP socket, so the caller opens the
//! stream with `ClientBuilder::connector`. Here that is tokio's `TcpStream`,
//! which WASI supports under `--cfg tokio_unstable`. Any tokio
//! `AsyncRead + AsyncWrite` stream works the same way.
//!
//! Needs the `wasm32-wasip2` target and `wasmtime` on `PATH`:
//!
//! ```bash
//! rustup target add wasm32-wasip2
//! cd examples/wasi
//! cargo run            # or: cargo run -- 127.0.0.1:4002
//! ```

use ibapi::Client;
use tokio::net::TcpStream;

// WASI has no threads: the client runs on a current-thread runtime.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = std::env::args().nth(1).unwrap_or_else(|| "127.0.0.1:4002".to_string());

    // Called again on every reconnect.
    let client = Client::builder()
        .connector(move || {
            let address = address.clone();
            async move {
                let stream = TcpStream::connect(address).await?;
                stream.set_nodelay(true)?;
                Ok(stream)
            }
        })
        .client_id(100)
        .connect()
        .await?;

    println!("server version: {}", client.server_version());
    println!("server time: {}", client.server_time().await?);
    Ok(())
}
