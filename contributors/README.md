## Overview

The API is designed to provide a robust, efficient, and flexible interface for communicating with TWS (Trader Workstation) or IB Gateway. This API allows developers to build trading applications in Rust, leveraging its performance and safety features. The architecture is built around threads and channels for sending requests and responses between the client and the TWS/IB Gateway.

The main thread handles user interactions with the API. The MessageBus runs on a dedicated thread. The MessageBus establishes the connection to TWS/IB Gateway, sends messages from the client to TWS/IB Gateway, and listens for and routes messages from TWS/IB Gateway to the client via channels.

## Core Components

### MessageBus

The MessageBus is a crucial component of the API, running on its own dedicated thread. Its responsibilities include:

* Establishing and maintaining the connection to TWS/IB Gateway
* Sending messages from the client to TWS/IB Gateway
* Listening for messages from TWS/IB Gateway
* Routing incoming messages to the appropriate client channels

Explore [MessageBus implementation](https://github.com/wboayue/rust-ibapi/blob/main/src/client/transport.rs) for more details.

### Client

The Client component runs on the main thread and provides the interface for user interactions with the API. It is responsible for:

* Encoding user requests into the format expected by TWS/IB Gateway
* Sending requests to the MessageBus
* Receiving responses from the MessageBus via channels
* Decoding responses and presenting them to the user

Explore [Client API](https://github.com/wboayue/rust-ibapi/blob/main/src/client.rs) for more details.

## Request and Response Handling

The API uses a combination of request IDs and channels to manage the flow of messages:

1. For requests with a request or order ID:

* The Client generates a unique ID for the request.
* The MessageBus creates a dedicated channel for responses based on the request ID
* Responses related to this request are sent through this channel

2. For requests without a request or order ID (TWS API limitation):

* The MessageBus uses predefined shared channels based on request type.
* Responses related to these requests are routed through these shared channels.

**Note**: Since these responses are not tied to specific request IDs, distinguishing between responses from concurrent requests of the same type requires careful handling.

The recommended application design is a separate Client instance per thread.

## Extending the API

To add new functionality to the API:

Define new message types in the protocol module
Implement encoding/decoding for new message types in the Client
Add new methods to the Client struct for the new functionality
Update the MessageBus to handle routing for the new message types
Provide examples and documentation for the new features

## Getting Started

### Run Coverage Report

https://github.com/taiki-e/cargo-llvm-cov

```bash
cargo install cargo-tarpaulin
cargo tarpaulin -o html
```

RUST_LOG=debug cargo run --bin find_contract_details

### Troubleshooting

`RUST_LOG=debug`
`IBAPI_RECORDING_DIR=/tmp`

## Creating and publishing releases.

1. Make sure build is clean and tests are passing.

```bash
cargo build --all-targets
cargo test
```

2. Update version number in [Cargo.toml](https://github.com/wboayue/rust-ibapi/blob/76033d170f2b87d55ed2cd96fef17bf124161d5f/Cargo.toml#L3) using [semantic versioning](https://semver.org/). Commit and push.

3. Create tag with new version number and push.

```bash
git tag v0.4.0
git push origin v0.4.0
```

4. [Create release](https://github.com/wboayue/rust-ibapi/releases/new) pointing to new tag.  Describe changes in release.

5. Publish to crates.io

```bash
cargo publish
```
