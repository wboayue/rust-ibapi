use ibapi::Client;

fn main() -> anyhow::Result<()> {
    let client = Client::connect("localhost:4002", 100)?;

    let market_rule_id = 12;
    let results = client.market_rule(market_rule_id)?;

    println!("rule: {results:?}");

    Ok(())
}
