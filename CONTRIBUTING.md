# Contributing to rust-ibapi

## Table of Contents
- [Overview](#overview)
- [Getting Started](#getting-started)
- [Coding Standards](#coding-standards)
- [Core Components](#core-components)
- [Request and Response Handling](#request-and-response-handling)
- [Extending the API](#extending-the-api)
- [Troubleshooting](#troubleshooting)
- [Creating and Publishing Releases](#creating-and-publishing-releases)

## Overview

The API is designed to provide a robust, efficient, and flexible interface for communicating with TWS (Trader Workstation) or IB Gateway. This API allows developers to build trading applications in Rust, leveraging its performance and safety features. The architecture is built around threads and channels for sending requests and responses between the client and the TWS.

The main thread handles user interactions with the API. The MessageBus runs on a dedicated thread. The MessageBus establishes the connection to TWS, sends messages from the client to TWS, and listens for and routes messages from TWS to the client via channels.

## Getting Started

1. [Install Rust](https://www.rust-lang.org/tools/install).

2. Install additional development tools:

* [cargo-tarpaulin](https://github.com/xd009642/tarpaulin) for code coverage analysis.
* [cargo-audit](https://rustsec.org/) for checking vulnerabilities in dependencies.

```bash
cargo install cargo-tarpaulin
cargo install cargo-audit
```

3. Create a [fork](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/working-with-forks/fork-a-repo) of the repository.

4. Clone your fork and make sure tests are working:

```bash
git clone https://github.com/<your-github-username>/rust-ibapi
cd rust-ibapi
cargo test
```

5. Set up your development environment:
   - We recommend using an IDE with Rust support, such as VS Code with the rust-analyzer extension.
   - Configure your IDE to use rustfmt and clippy for code formatting and linting.

6. Make your changes.

* Ensure tests are still passing and coverage hasn't dropped:

```bash
cargo test
cargo tarpaulin -o html
```

* The coverage report will be saved as tarpaulin-report.html. Open it in your browser to view the coverage details.

7. Submit a Pull Request

* Follow GitHub's guide on [creating a pull request from a fork](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/proposing-changes-to-your-work-with-pull-requests/creating-a-pull-request-from-a-fork).

## Coding Standards

We follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/). Please ensure your code adheres to these guidelines. Use `cargo fmt` to format your code and `cargo clippy` to catch common mistakes and improve your Rust code.

## Core Components

### MessageBus

The `MessageBus` is a crucial component of the API, running on its own dedicated thread. Its responsibilities include:

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
* The MessageBus creates a dedicated channel for responses based on the request ID.
* Responses related to this request are sent through these channels.

2. For requests without a request or order ID (due to TWS API design):

* The MessageBus creates a shared channel for responses of that request type.
* Responses related to these requests are routed through these shared channels.
* **Note**: Since these responses are not tied to specific request IDs, distinguishing between responses from concurrent requests of the same type requires careful handling.

The recommended application design is a separate Client instance per thread to avoid message routing issues.

## Extending the API

1. Define the new API method

* The API exposed to the user is defined on the [Client struct](https://github.com/wboayue/rust-ibapi/blob/main/src/client.rs#L33).
* Define the interface for the new API on the Client struct. The actual implementation of the API is delegated to modules grouped by accounts, contracts, market data, orders and news.
* Include a docstring describing the API that includes and example of the API usage. [for example](https://github.com/wboayue/rust-ibapi/blob/main/src/client.rs#L226).

2. Ensure message identifiers ar defined.

* Make sure the appropriate [incoming message](https://github.com/wboayue/rust-ibapi/blob/main/src/messages.rs#L15) and [outgoing message](https://github.com/wboayue/rust-ibapi/blob/main/src/messages.rs#L222) identifiers are defined.
* Message identifiers for [incoming messages](https://github.com/InteractiveBrokers/tws-api/blob/master/source/csharpclient/client/IncomingMessage.cs) and [outgoing messages](https://github.com/InteractiveBrokers/tws-api/blob/master/source/csharpclient/client/OutgoingMessages.cs) can be found in the interactive brokers codebase.

3. Update the message type to request ID map.

* When processing messages received from TWS, the request id needs to be determined. This is not the same for all messages.
* A [map of message type to request id position](https://github.com/wboayue/rust-ibapi/blob/main/src/messages.rs#L199) is maintained and may need to be updated.

4. Add an implementation for the API in the appropriate group.

* Add an implementation for the API in the appropriate group: accounts, contracts, market data, orders or news.
* The implementation will provide an encoder to convert the request to the TWS format
* Send the message using the `MessageBus`.
   * Messages with a request id are sent using [send_generic_message](https://github.com/wboayue/rust-ibapi/blob/main/src/client/transport.rs#L26).
   * Messages without a request id are sent using message type methods. e.g. [request_next_order_id](https://github.com/wboayue/rust-ibapi/blob/main/src/client/transport.rs#L29)

5. Implement a decoder for the response.

* Implement a decoder for the response received from the `MessageBus`.
* Responses contain a channel that can you used to read the results as they become available.
* For APIs that return a single result, they may simply decode and return the result.
* For a collection of results, return a Subscription that can be used to iterate over results.

6. Add test cases.

* Add test cases for the new functionality.
* Run coverage analysis. Your addition should improve or maintain the [current coverage](https://coveralls.io/github/wboayue/rust-ibapi?branch=main).
* Use `cargo tarpaulin` to generate coverage reports.

7. Add an example.

* Add an example showing the API usage to the [examples folder](https://github.com/wboayue/rust-ibapi/tree/main/examples).
* Ensure your example is well-documented and can help users understand how to use the new API method.

## Troubleshooting

The following environment variables are useful for troubleshooting:

* `RUST_LOG` - Changes the log level. Possible values are `trace`, `debug`, `info`, `warn`, `error`.
* `IBAPI_RECORDING_DIR` - If this is set, the library logs messages between the library and TWS to the specified directory.

For example, the following sets the log level to `debug` and instructs the library to log messages between it and TWS to `/tmp/tws-messages`:

```bash
RUST_LOG=debug IBAPI_RECORDING_DIR=/tmp/tws-messages cargo run --bin find_contract_details
```

## Creating and publishing releases.

1. Ensure build is clean and tests are passing.

```bash
cargo build --all-targets
cargo test
```

2. Update version number.

* Update version number in [Cargo.toml](https://github.com/wboayue/rust-ibapi/blob/main/Cargo.toml#L3) using [semantic versioning](https://semver.org/).
* Commit and push your changes.

3. Create tag with new version number.

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

4. Create a release.

* [Create release](https://github.com/wboayue/rust-ibapi/releases/new) pointing to new tag.
* Describe changes in release.

5. Publish to crates.io.

* Before publishing, run a dry run to catch any issues:

```bash
cargo publish --dry-run
```

* If everything looks good, publish the crate:

```bash
cargo publish
```
