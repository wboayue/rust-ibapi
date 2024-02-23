## Architecture

client -- message bus

encoders / decoders

channels

encode message -> create response channel -> send message -> return response channel

channel cleanup

### Add new message

generic vs global messages


## Run Coverage Report

https://github.com/taiki-e/cargo-llvm-cov

```bash
cargo install cargo-tarpaulin
cargo tarpaulin -o html
```

RUST_LOG=debug cargo run --bin find_contract_details

## Troubleshooting

`RUST_LOG=debug`
`IBAPI_RECORDING_DIR=/tmp`


## Add example

# Covergae better than start
