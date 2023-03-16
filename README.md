# rust-ibapi
An implementation of the Interactive Brokers API for Rust

[![test](https://github.com/wboayue/rust-ibapi/workflows/ci/badge.svg)](https://github.com/wboayue/rust-ibapi/actions/workflows/ci.yml)
<!-- [![codecov](https://codecov.io/gh/wboayue/ibapi/branch/main/graph/badge.svg)](https://codecov.io/gh/wboayue/ibapi) -->


https://interactivebrokers.github.io/tws-api/introduction.html

RUST_LOG=debug 

// https://github.com/InteractiveBrokers/tws-api/blob/master/source/csharpclient/client/EClient.cs
// https://github.com/InteractiveBrokers/tws-api/blob/master/source/csharpclient/client/EDecoder.cs#L733

https://github.com/InteractiveBrokers/tws-api/blob/5cb24aea5cef9d315985a7b13dea7efbcfe2b16a/source/csharpclient/client/IBParamsList.cs

RUST_LOG=debug cargo run --bin find_contract_details

feat: (new feature for the user, not a new feature for build script)
fix: (bug fix for the user, not a fix to a build script)
docs: (changes to the documentation)
style: (formatting, missing semi colons, etc; no production code change)
refactor: (refactoring production code, eg. renaming a variable)
test: (adding missing tests, refactoring tests; no production code change)
chore: (updating grunt tasks etc; no production code change)

# Run Coverage Report

https://github.com/taiki-e/cargo-llvm-cov

```bash
cargo +stable install cargo-llvm-cov --locked

cargo llvm-cov --open
```

# Debugging

`IBAPI_RECORDING_DIR`

IBAPI_RECORDING_DIR=/tmp
```
RUST_LOG=debug IBAPI_RECORDING_DIR=/tmp/records cargo run --bin find_contract_details
```

https://rust-lang.github.io/rustfmt/?version=v1.5.1&search=