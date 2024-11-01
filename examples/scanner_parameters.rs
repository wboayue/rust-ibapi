use ibapi::Client;

fn main() {
    env_logger::init();

    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");

    let parameters = client.scanner_parameters().expect("request scanner parameters failed");
    println!("{}", parameters);
}
