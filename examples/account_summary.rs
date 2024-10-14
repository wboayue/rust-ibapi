use ibapi::accounts::{AccountSummaries, AccountSummaryTags};
use ibapi::Client;

fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let group = "All";

    let subscription = client
        .account_summary(group, AccountSummaryTags::ALL)
        .expect("error requesting account summary");
    for update in &subscription {
        match update {
            AccountSummaries::Summary(summary) => println!("{summary:?}"),
            AccountSummaries::End => subscription.cancel().expect("cancel failed"),
        }
    }
}
