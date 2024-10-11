use ibapi::Client;

pub fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let account = "U1234567";
    let subscription = client.positions_multi(Some(account), None).expect("error requesting positions by model");
    for position in subscription {
        println!("{position:?}")
    }
}
