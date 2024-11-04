use ibapi::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let filter = "<contract>76792991</contract>"; // TSLA

    let subscription = client
        .wsh_event_data_by_filter(filter, None, None)
        .expect("request wsh event data failed");

    for event_data in subscription {
        println!("{:?}", event_data);
    }
}
