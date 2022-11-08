use anyhow;
use ibapi;

fn main() -> anyhow::Result<()> {
    let port = 4002;
    let client_id = 100;
    let host = "localhost";

    let client = ibapi::client::connect(host, port, client_id)?;

    println!("Client: {:?}", client);

    Ok(())
}
