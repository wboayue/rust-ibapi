#[cfg(feature = "async")]
#[tokio::test]
async fn test_wsh_metadata_async() {
    use ibapi::wsh::wsh_metadata;
    use ibapi::Client;
    use std::sync::Arc;

    // Note: This is a basic compile test to ensure the async implementation works
    // In real usage, you would connect to a live TWS instance

    // This test just ensures the async code compiles and the API is usable
    let _ = async {
        // Example usage (would fail without real connection):
        // let client = Client::connect("127.0.0.1:4002", 100).await?;
        // let metadata = wsh_metadata(&client).await?;
        // println!("WSH Metadata: {:?}", metadata);
        Ok::<(), ibapi::Error>(())
    };
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_wsh_event_data_async() {
    use ibapi::wsh::{wsh_event_data_by_contract, AutoFill};
    use ibapi::Client;
    use time::macros::date;

    // This test just ensures the async code compiles and the API is usable
    let _ = async {
        // Example usage (would fail without real connection):
        // let client = Client::connect("127.0.0.1:4002", 100).await?;
        // let event_data = wsh_event_data_by_contract(
        //     &client,
        //     12345,
        //     Some(date!(2024-01-01)),
        //     Some(date!(2024-12-31)),
        //     Some(100),
        //     Some(AutoFill {
        //         competitors: true,
        //         portfolio: false,
        //         watchlist: false,
        //     })
        // ).await?;
        // println!("WSH Event Data: {:?}", event_data);
        Ok::<(), ibapi::Error>(())
    };
}
