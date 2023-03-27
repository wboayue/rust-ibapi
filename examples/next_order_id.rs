use ibapi::Client;

fn main() -> anyhow::Result<()> {
    let client = Client::connect("localhost:4002")?;

    let order_id = client.next_valid_order_id()?;
    println!("Next valid order id: {order_id}");

    Ok(())
}
