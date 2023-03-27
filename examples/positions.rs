use ibapi::{Client, Error};

fn main() -> Result<(), Error> {
    let client = Client::connect("localhost:4002")?;

    let positions = client.positions()?;
    for position in positions {
        println!("position: {position:?}")
    }

    Ok(())
}
