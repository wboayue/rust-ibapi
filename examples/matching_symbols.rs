use ibapi::Client;

fn main() -> anyhow::Result<()> {
    let client = Client::connect("localhost:4002")?;

    let pattern = "TSLA";
    let results = client.matching_symbols(&pattern)?;
    for result in results {
        println!("contract: {result:?}");
    }

    Ok(())
}
