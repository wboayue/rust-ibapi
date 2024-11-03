use ibapi::{wsh::AutoFill, Client};
use time::macros::date;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let contract_id = 76792991; // TSLA
    let start_date = None;
    let end_date = None;
    let limit = None;
    let auto_fill = None;

    let subscription = client
        .wsh_event_data_by_contract(contract_id, start_date, end_date, limit, auto_fill)
        .expect("request wsh event data failed");

    for event_data in subscription {
        println!("{:?}", event_data);
    }
}
