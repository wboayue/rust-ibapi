use ibapi::Client;

#[tokio::test]
async fn connect_to_gateway() {
    let client = Client::connect("127.0.0.1:4002", 100)
        .await
        .expect("connection failed");

    assert!(client.server_version() > 0);
    assert!(client.connection_time().is_some());

    let time = client.server_time().await.expect("failed to get server time");
    assert!(time.year() >= 2025);
}
