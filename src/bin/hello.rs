use ibapi;

fn main() {
    let port = 4002;
    let client_id = 100;
    let host = "localhost";

    let client = ibapi::client::connect(host, port, client_id).unwrap();

    println!("Client: {:?}", client);
}
