use ibapi::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let subscription = client.wsh_metadata().expect("request wsh metadata failed");
    for metadata in subscription {
        println!("{:?}", metadata);
    }
}
