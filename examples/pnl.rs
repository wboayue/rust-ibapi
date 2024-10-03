use ibapi::Client;

fn main() {
    let client_url = std::env::var("CLIENT_URL").expect("CLIENT_URL must be set");
    let account_id = std::env::var("ACCOUNT_ID").expect("ACCOUNT_ID must be set");

    let client = Client::connect(&client_url, 919).expect("connection failed");

    // let pnl = client.pnl(&account_id).expect("request failed");
    // println!("PnL: {:?}", pnl);
}
