[![Build](https://github.com/wboayue/rust-ibapi/workflows/ci/badge.svg)](https://github.com/wboayue/rust-ibapi/actions/workflows/ci.yml)
[![License:MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![crates.io](https://img.shields.io/crates/v/twsapi.svg)](https://crates.io/crates/ibapi)
[![Documentation](https://img.shields.io/badge/Documentation-green.svg)](https://docs.rs/ibapi/0.1.0/ibapi)

## Introduction

An implementation of the Interactive Brokers [TWS API](https://interactivebrokers.github.io/tws-api/introduction.html) for Rust. The official TWS API is an event driven API. This implementation provides a synchronous API that simplifies the development of trading strategies.

This is a work in progress and targets support for TWS API 10.20. The primary reference for the implementation is the CSharp code of the [official API](https://github.com/InteractiveBrokers/tws-api-public).

List of open issues are tracked [here](https://github.com/wboayue/rust-ibapi/issues). If you run into an issue or need a missing feature, check the [issues list](https://github.com/wboayue/rust-ibapi/issues) first and report the issue if it is not already tracked.

Contributions are welcome. Open a pull request.

## Installation

Run the following Cargo command in your project directory:

```
cargo add ibapi
```

Or add the following line to your Cargo.toml:

```
ibapi = "0.1.0"
```

## Example 

## Documentation

API documentation is located [here](https://docs.rs/ibapi/0.1.0/ibapi)
