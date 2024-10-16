use ibapi::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).unwrap();

    let order_id = client.next_valid_order_id().unwrap();
    println!("Next valid order id: {order_id}");
}
