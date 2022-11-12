use anyhow;

#[derive(Debug)]
pub struct BasicClient<'a> {
    host: &'a str,
    port: i32,
    client_id: i32,
}

pub struct Packet {}
pub struct PacketIterator {}

pub trait Client {
    fn next_request_id(&self) -> i32;
    fn send_packet(&self, packet: &Packet) -> i32;
    fn receive_packet(&self, request_id: i32) -> Packet;
    fn receive_packets(&self, request_id: i32) -> PacketIterator;
}

// receive_packet
// receive_packets

pub fn connect(host: &str, port: i32, client_id: i32) -> anyhow::Result<BasicClient> {
    println!("Connect, world!");
    Ok(BasicClient {
        host,
        port,
        client_id,
    })
}
