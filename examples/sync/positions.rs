//! Positions example
//!
//! # Usage
//!
//! ```bash
//! cargo run --features sync --example positions
//! ```

use ibapi::accounts::PositionUpdate;
use ibapi::client::blocking::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let positions = client.positions().expect("request failed");
    while let Some(position_update) = positions.next() {
        match position_update {
            PositionUpdate::Position(position) => {
                println!(
                    "{:4} {:4} {} @ {}",
                    position.position, position.contract.symbol, position.contract.contract_id, position.average_cost
                )
            }
            PositionUpdate::PositionEnd => {
                println!("PositionEnd");
                // all positions received. could continue listening for new additions or cancel.
                positions.cancel();
                break;
            }
        }
    }
}
