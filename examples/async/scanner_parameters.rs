use ibapi::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Connect to IB Gateway or TWS
    let client = Client::connect("127.0.0.1:4002", 100).await?;

    println!("=== Requesting Scanner Parameters ===");

    let xml_parameters = client.scanner_parameters().await?;

    println!("Scanner parameters XML length: {} bytes", xml_parameters.len());

    // The XML contains all available scanner parameters
    // In a real application, you would parse this XML to extract:
    // - Available scan types
    // - Valid locations
    // - Valid instruments
    // - Available filters

    // Print first 500 characters as a sample
    if xml_parameters.len() > 500 {
        println!("\nFirst 500 characters of XML:");
        println!("{}", &xml_parameters[..500]);
        println!("... (truncated)");
    } else {
        println!("\nFull XML:");
        println!("{}", xml_parameters);
    }

    Ok(())
}
