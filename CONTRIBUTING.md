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

## Creating and publishing releases.

1. Make sure build is clean and tests are passing.

```bash
cargo build --all-targets
cargo test
```

2. Update version number in [Cargo.toml](https://github.com/wboayue/rust-ibapi/blob/76033d170f2b87d55ed2cd96fef17bf124161d5f/Cargo.toml#L3) using [semantic versioning](https://semver.org/). Commit and push.

3. Create tag with new version number and push.

```bash
git tag v0.4.0 main
git push origin tag v0.4.0
```

4. [Create release](https://github.com/wboayue/rust-ibapi/releases/new) pointing to new tag.  Describe changes in release.

5. Publish to crates.io

```bash
cargo publish
```