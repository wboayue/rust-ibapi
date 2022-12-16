# rust-ibapi
An implementation of the Interactive Brokers API for Rust

https://interactivebrokers.github.io/tws-api/introduction.html

RUST_LOG=debug 

// https://github.com/InteractiveBrokers/tws-api/blob/master/source/csharpclient/client/EClient.cs
// https://github.com/InteractiveBrokers/tws-api/blob/master/source/csharpclient/client/EDecoder.cs#L733

RUST_LOG=debug cargo run --bin find_contract_details


TODO: fix request/response channel leak
