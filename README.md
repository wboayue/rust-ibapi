[![Build](https://github.com/wboayue/rust-ibapi/workflows/ci/badge.svg)](https://github.com/wboayue/rust-ibapi/actions/workflows/ci.yml)
[![License:MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

<!-- [![codecov](https://codecov.io/gh/wboayue/ibapi/branch/main/graph/badge.svg)](https://codecov.io/gh/wboayue/ibapi) -->

# Introduction

An implementation of the Interactive Brokers API for Rust. There are other Rust implementations but the tend to port event driven style of the official Interactive Brokers API. I find code written in a synchronous style easier to understand when implmenting
algorithmic trading solutions.

In progress. Issue list here. Contributions are welcome.

## Installation

Run the following Cargo command in your project directory:

```bash
cargo add ibapi
```

Or add the following line to your Cargo.toml:

```
ibapi = "0.1.0"
```

## Example 

## Documentation

https://github.com/xd009642/tarpaulin
coveralls

https://interactivebrokers.github.io/tws-api/introduction.html


// https://github.com/InteractiveBrokers/tws-api/blob/master/source/csharpclient/client/EClient.cs
// https://github.com/InteractiveBrokers/tws-api/blob/master/source/csharpclient/client/EDecoder.cs#L733

https://github.com/InteractiveBrokers/tws-api/blob/5cb24aea5cef9d315985a7b13dea7efbcfe2b16a/source/csharpclient/client/IBParamsList.cs

RUST_LOG=debug cargo run --bin find_contract_details

# Run Coverage Report

https://github.com/taiki-e/cargo-llvm-cov

```bash
cargo install cargo-tarpaulin
cargo tarpaulin -o html
```

# Troubleshooting

## Logs
RUST_LOG=debug 

`IBAPI_RECORDING_DIR`

## Message spy

IBAPI_RECORDING_DIR=/tmp
```
RUST_LOG=debug IBAPI_RECORDING_DIR=/tmp/records cargo run --bin find_contract_details
```

https://rust-lang.github.io/rustfmt/?version=v1.5.1&search=