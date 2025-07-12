use ibapi::contracts::{Contract, SecurityType};
use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS
    let client = Client::connect("127.0.0.1:4002", 100).await?;

    // Create an option contract
    let contract = Contract {
        symbol: "AAPL".to_string(),
        security_type: SecurityType::Option,
        exchange: "SMART".to_string(),
        currency: "USD".to_string(),
        strike: 150.0,
        right: "C".to_string(),                                    // Call option
        last_trade_date_or_contract_month: "20250117".to_string(), // January 17, 2025
        ..Default::default()
    };

    // Calculate implied volatility given option price and underlying price
    let option_price = 8.50; // Current option price
    let underlying_price = 155.0; // Current underlying stock price

    println!("=== Calculating Implied Volatility ===");
    println!(
        "Contract: {} {} {} @ {}",
        contract.symbol, contract.last_trade_date_or_contract_month, contract.right, contract.strike
    );
    println!("Option Price: ${:.2}", option_price);
    println!("Underlying Price: ${:.2}", underlying_price);

    match client.calculate_implied_volatility(&contract, option_price, underlying_price).await {
        Ok(computation) => {
            println!("\n=== Results ===");
            if let Some(iv) = computation.implied_volatility {
                println!("Implied Volatility: {:.1}%", iv * 100.0);
            }
            if let Some(delta) = computation.delta {
                println!("Delta: {:.4}", delta);
            }
            if let Some(gamma) = computation.gamma {
                println!("Gamma: {:.4}", gamma);
            }
            if let Some(vega) = computation.vega {
                println!("Vega: {:.4}", vega);
            }
            if let Some(theta) = computation.theta {
                println!("Theta: {:.4}", theta);
            }
            if let Some(option_price_calc) = computation.option_price {
                println!("Calculated Option Price: ${:.2}", option_price_calc);
            }
            if let Some(pv_dividend) = computation.present_value_dividend {
                println!("PV Dividend: ${:.2}", pv_dividend);
            }
        }
        Err(e) => eprintln!("Error calculating implied volatility: {e:?}"),
    }

    Ok(())
}
