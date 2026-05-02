//! One-shot TWS-style handshake listener for `Client::connect*` tests.
//!
//! Bound to `127.0.0.1:0` (kernel-assigned port). Accepts a single connection,
//! consumes the client's `API\0` + version range + `start_api` writes, replays
//! the supplied frames length-prefixed, then drains further writes until the
//! client closes the stream so the dispatcher's read-timeout loop sees a clean
//! shutdown when `Client` drops.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread::{self, JoinHandle};

use crate::messages::encode_raw_length;

pub(crate) fn spawn_handshake_listener(frames: Vec<Vec<u8>>) -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let addr = listener.local_addr().expect("local_addr");
    let handle = thread::spawn(move || {
        let (mut stream, _) = match listener.accept() {
            Ok(s) => s,
            Err(_) => return,
        };

        // Magic token: "API\0" (4 bytes, no length prefix).
        let mut magic = [0u8; 4];
        if stream.read_exact(&mut magic).is_err() {
            return;
        }

        // Version range: length-prefixed string.
        if read_length_prefixed(&mut stream).is_err() {
            return;
        }

        // Replay scripted handshake response + post-handshake frames.
        for frame in &frames {
            let packet = encode_raw_length(frame);
            if stream.write_all(&packet).is_err() {
                return;
            }
        }

        // Drain further writes (start_api + any later traffic) until the
        // client closes. read returning 0 bytes signals the client's drop.
        let mut sink = [0u8; 1024];
        while stream.read(&mut sink).map(|n| n > 0).unwrap_or(false) {}
    });
    (addr, handle)
}

fn read_length_prefixed(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes)?;
    let len = u32::from_be_bytes(len_bytes) as usize;
    let mut data = vec![0u8; len];
    stream.read_exact(&mut data)?;
    Ok(data)
}
