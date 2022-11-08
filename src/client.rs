use std::fmt::Error;

#[derive(Debug)]
pub struct Client<'a> {
    host: &'a str,
    port: i32,
    client_id: i32
}

pub fn connect(host: &str, port: i32, client_id: i32) -> Result<Client, Error> {
    println!("Connect, world!");
    Ok(Client{host, port, client_id})
}
