# Contributing to rust-ibapi

## Table of Contents
- [Overview](#overview)
- [Getting Started](#getting-started)
- [Coding Standards](#coding-standards)
- [Core Components](#core-components)
- [Request and Response Handling](#request-and-response-handling)
- [Extending the API](#extending-the-api)
- [Commit Message Guidelines](#commit-message-guidelines)
- [Pull Request Process](#pull-request-process)
- [Documentation](#documentation)
- [Reporting Bugs](#reporting-bugs)
- [Feature Requests](#feature-requests)
- [Troubleshooting](#troubleshooting)
- [Community](#community)
- [Creating and Publishing Releases](#creating-and-publishing-releases)
- [License](#license)
- [Acknowledgements](#acknowledgements)

## Overview

The API is designed to provide a robust, efficient, and flexible interface for communicating with TWS (Trader Workstation) or IB Gateway. This API allows developers to build trading applications in Rust, leveraging its performance and safety features. The architecture is built around threads and channels for sending requests and responses between the client and the TWS.

The main thread handles user interactions with the API. The MessageBus runs on a dedicated thread. The MessageBus establishes the connection to TWS, sends messages from the client to TWS, and listens for and routes messages from TWS to the client via channels.

## Getting Started

1. [Install Rust](https://www.rust-lang.org/tools/install).

2. Install additional development tools:
   ```bash
   cargo install cargo-tarpaulin
   cargo install cargo-audit
   ```

3. Create a [fork](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/working-with-forks/fork-a-repo) of the repository.

4. Clone your fork and make sure tests are working:
   ```bash
   git clone https://github.com/<your fork>/rust-ibapi
   cd rust-ibapi
   cargo test
   ```

5. Set up your development environment:
   - We recommend using an IDE with Rust support, such as VS Code with the rust-analyzer extension.
   - Configure your IDE to use rustfmt and clippy for code formatting and linting.

6. Make your changes. Ensure tests are still passing and coverage hasn't dropped:
   ```bash
   cargo test
   cargo tarpaulin -o html
   ```

7. Submit a [pull request](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/proposing-changes-to-your-work-with-pull-requests/creating-a-pull-request-from-a-fork).

## Coding Standards

We follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/). Please ensure your code adheres to these guidelines. Use `cargo fmt` to format your code and `cargo clippy` to catch common mistakes and improve your Rust code.

## Core Components

### MessageBus

The MessageBus is a crucial component of the API, running on its own dedicated thread. Its responsibilities include:

* Establishing and maintaining the connection to TWS
* Sending messages from the client to TWS
* Listening for messages from TWS
* Routing incoming messages to the appropriate client channels

Explore [MessageBus implementation](https://github.com/wboayue/rust-ibapi/blob/main/src/client/transport.rs) for more details.

### Client

The Client component runs on the main thread and provides the interface for user interactions with the API. It is responsible for:

* Encoding user requests into the format expected by TWS
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

1. The API exposed to the user is defined on the Client struct. The API implementation is delegated to modules grouped by accounts, contracts, market data, orders and news. Define the news API on the Client struct. Include a docstring describing the API that includes and example of the API usage. [for example](https://github.com/liuzix/rust-ibapi/blob/cc6287ba73e705324908adbd37dbd32a565dd1c1/src/client.rs#L226).

2. Make sure the appropriate [incoming message](https://github.com/wboayue/rust-ibapi/blob/01a521d008a8269720d2a5a823958823ff37cbe2/src/messages.rs#L15) and [outgoing message](https://github.com/wboayue/rust-ibapi/blob/01a521d008a8269720d2a5a823958823ff37cbe2/src/messages.rs#L222) identifiers are defined. Message identifiers for [incoming messags](https://github.com/InteractiveBrokers/tws-api/blob/master/source/csharpclient/client/IncomingMessage.cs) and [outgoing messages](https://github.com/InteractiveBrokers/tws-api/blob/master/source/csharpclient/client/OutgoingMessages.cs) can be found in the interactive brokers codebase.

3. When processing messages received from TWS/IB Gateway the request id is extracted. This is not the same for all messages. A [map of message type to request id](https://github.com/wboayue/rust-ibapi/blob/289abc31432d768c78db2dfe5ef3cf66b174d91f/src/messages.rs#L199) is maintained an will need to be updated.

4. Add an implementation for the API in the appropriate group.
accounts, contracts, market data, orders and news. The implementation
will provide an encoder to covert the request to the TWS format format. Send the message using the MessageBus. Message with a request id are sent using [send_generic_message](https://github.com/wboayue/rust-ibapi/blob/289abc31432d768c78db2dfe5ef3cf66b174d91f/src/client/transport.rs#L26). Messages without a request id are sent using message type methods. e.g. [request_next_order_id](https://github.com/wboayue/rust-ibapi/blob/289abc31432d768c78db2dfe5ef3cf66b174d91f/src/client/transport.rs#L29) 

5. Implement a decode using the response. Response returns a channel that can you used to read the results as they become available. Implement a decoder. For a single item the API can just return the result. Collection of items decoder returns a subscription used to iterate over results.

6. Add test cases for the new functionality. Run coverage. Your addition should improve of maintain the [current coverage](https://coveralls.io/github/wboayue/rust-ibapi?branch=main). 

7. Add an example showing the API usage to the [examples folder](https://github.com/wboayue/rust-ibapi/tree/main/examples).


### Troubleshooting

The following environment variables are useful for troubleshooting.

* RUST_LOG - changes the log level
* IBAPI_RECORDING_DIR - If this is set the library logs messages between the library and TWS to the specified directory.

For example, the followings set the log level to `debug` and instructs the library to log messages between it and TWS to `/tmp/tws-messages`

```bash
RUST_LOG=debug IBAPI_RECORDING_DIR=/tmp/tws-messages cargo run --bin find_contract_details
```

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
