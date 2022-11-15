use anyhow;
use env_logger;
use log::{info};

use ibapi::client::{BasicClient};


fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut client = BasicClient::connect("odin:4002");
    
    info!("Connected {:?}", client);

    Ok(())
}
