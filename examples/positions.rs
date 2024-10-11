use ibapi::{accounts::PositionResponse, Client};

fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let mut positions = client.positions().expect("request failed");
    while let Some(position_update) = positions.next() {
        match position_update {
            PositionResponse::Position(position) => {
                println!("{:4} {:4} @ {}", position.position, position.contract.symbol, position.average_cost)
            }
            PositionResponse::PositionEnd => {
                println!("PositionEnd");
                // all positions received. could continue listening for new additions or cancel.
                positions.cancel();
            }
        }
    }
}
