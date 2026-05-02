//! One-shot TWS-style handshake listener for `AsyncConnection::connect*` and
//! `Client::connect*` (async) tests. Mirrors `transport/sync/test_listener.rs`.

use std::net::SocketAddr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use crate::messages::encode_raw_length;

pub(crate) async fn spawn_handshake_listener(frames: Vec<Vec<u8>>) -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind 127.0.0.1:0");
    let addr = listener.local_addr().expect("local_addr");
    let handle = tokio::spawn(async move {
        let (mut stream, _) = match listener.accept().await {
            Ok(s) => s,
            Err(_) => return,
        };

        let mut magic = [0u8; 4];
        if stream.read_exact(&mut magic).await.is_err() {
            return;
        }

        let mut len_bytes = [0u8; 4];
        if stream.read_exact(&mut len_bytes).await.is_err() {
            return;
        }
        let len = u32::from_be_bytes(len_bytes) as usize;
        let mut version_range = vec![0u8; len];
        if stream.read_exact(&mut version_range).await.is_err() {
            return;
        }

        for frame in &frames {
            let packet = encode_raw_length(frame);
            if stream.write_all(&packet).await.is_err() {
                return;
            }
        }
        let _ = stream.flush().await;

        // Drain further writes (start_api + later traffic) until the client
        // closes the stream.
        let mut sink = [0u8; 1024];
        while stream.read(&mut sink).await.map(|n| n > 0).unwrap_or(false) {}
    });
    (addr, handle)
}
