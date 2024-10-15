/// Example of connecting to TWS.
use ibapi::Client;

fn main() {
    env_logger::init();

    let connection_url = "127.0.0.1:4002";

    let _client = Client::connect(connection_url, 100).expect("connection to TWS failed!");
    println!("Successfully connected to TWS at {connection_url}");
}
