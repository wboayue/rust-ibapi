use ibapi::accounts::AccountUpdateMulti;
use ibapi::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let account = Some("DU1234567");

    let subscription = client
        .account_updates_multi(account, None)
        .expect("error requesting account updates multi");
    for update in &subscription {
        println!("{update:?}");

        // stop after full initial update
        if let AccountUpdateMulti::End = update {
            subscription.cancel();
        }
    }
}
