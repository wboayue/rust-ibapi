use ibapi::Client;

fn main() {
    let client = Client::connect("127.0.0.1:4002", 100).expect("connection failed");
    
    
    let family_codes = client.family_codes().expect("request failed");
    
    for family_code in family_codes {
        println!("{:4} {:4}", family_code[0].account_id, family_code[0].family_code)
    }

    
}