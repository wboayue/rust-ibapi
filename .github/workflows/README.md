# CI/CD Workflows

This directory contains the GitHub Actions workflows for the rust-ibapi project.

## Workflows

### ci.yml
The main CI workflow that runs on every push and pull request to the main branch. It includes:

#### `ci` Job
- **Matrix**: One leg per feature configuration: `async` (default, no flags), `sync`
  (`--no-default-features --features sync`), `all-features` (`--all-features`)
- **Steps** (every one of these runs once per leg, three times in total):
  - `cargo fmt -- --check` — the format check is a step inside the matrix, not a
    separate job, so it repeats per leg even though formatting is feature-independent
  - Build the library with the leg's features
  - `cargo clippy --all-targets <flags> -- -D warnings` — warnings as errors, across
    tests and examples as well as the library
  - Run all tests
  - Build all examples to ensure they compile
  - Build documentation with `cargo doc --no-deps`
  - Check that benches compile (failures here are tolerated)

#### `basic-checks` Job
- Runs once, outside the feature matrix
- Validates `Cargo.toml` with `cargo metadata`
- Validates the `docs/rules/` knowledge graph via `./tools/check-rules-graph.sh`
- Runs `cargo audit` for a security advisory scan (failures are tolerated)

### coverage.yml
Runs after successful CI workflow completion:
- Generates code coverage for both `sync` and `async` features
- Uses cargo-llvm-cov on nightly for coverage measurement (nightly is required for `--doctests`)
- Uploads results to Coveralls in parallel
- Merges coverage from both feature sets

## Feature Testing

The workflows test three feature configurations:

1. **Async**:
   ```bash
   cargo build
   cargo test
   cargo build --examples
   ```

2. **Sync**:
   ```bash
   cargo build --no-default-features --features sync
   cargo test --no-default-features --features sync
   cargo build --examples --no-default-features --features sync
   ```

3. **All features**:
   ```bash
   cargo build --all-features
   cargo test --all-features
   cargo build --examples --all-features
   ```

Running all three sets locally reproduces the `ci` job's coverage of the feature matrix.

## Caching

All workflows use GitHub Actions cache to speed up builds:
- Caches cargo registry and git dependencies
- Caches build artifacts in the target directory
- Uses separate cache keys for different features to avoid conflicts