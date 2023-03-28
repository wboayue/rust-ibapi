use ibapi::Client;

fn main() {
    let client = Client::connect("localhost:4002").expect("connection failed");

    let positions = client.positions().expect("request failed");
    for position in positions {
        println!("{:4} {:4} @ {}", position.position, position.contract.symbol, position.average_cost)
    }
}
