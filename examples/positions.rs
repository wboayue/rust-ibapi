use ibapi::{Client, IbApiError};

fn main() -> Result<(), IbApiError> {
    let client = Client::connect("localhost:4002")?;

    let positions = client.positions()?;
    for position in positions {
        println!("position: {position:?}")
    }

    Ok(())
}
