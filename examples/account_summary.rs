use ibapi::accounts::{AccountSummaryTags, AccountUpdate};
use ibapi::Client;

fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let group = "All";

    let subscription = client
        .account_summary(group, AccountSummaryTags::ALL)
        .expect("error requesting account summary");
    for update in &subscription {
        match update {
            AccountUpdate::Summary(summary) => println!("{summary:?}"),
            AccountUpdate::End => subscription.cancel().expect("cancel failed"),
        }
    }
}
