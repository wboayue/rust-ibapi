use ibapi::Client;

fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    let server_time = client.server_time().expect("error requesting server time");
    println!("server time: {server_time:?}");
}
