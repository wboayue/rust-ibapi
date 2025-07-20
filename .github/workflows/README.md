# CI/CD Workflows

This directory contains the GitHub Actions workflows for the rust-ibapi project.

## Workflows

### ci.yml
The main CI workflow that runs on every push and pull request to the main branch. It includes:

#### Test Job
- **Matrix**: Tests both `sync` and `async` features
- **Steps**:
  - Build the library with appropriate features
  - Run all tests
  - Build all examples to ensure they compile

#### Clippy Job
- **Matrix**: Runs clippy for both `sync` and `async` features
- **Steps**:
  - Runs clippy with warnings as errors (`-D warnings`)
  - Checks all targets including tests and examples

#### Format Job
- Runs once (formatting is feature-independent)
- Checks that all code is properly formatted with `cargo fmt`

#### Documentation Job
- **Matrix**: Builds docs for both `sync` and `async` features
- Ensures documentation compiles without errors

### coverage.yml
Runs after successful CI workflow completion:
- Generates code coverage for both `sync` and `async` features
- Uses cargo-tarpaulin for coverage measurement
- Uploads results to Coveralls in parallel
- Merges coverage from both feature sets

## Feature Testing

The workflows test both feature configurations:

1. **Sync**:
   ```bash
   cargo build --features sync
   cargo test --features sync
   cargo build --examples --features sync
   ```

2. **Async**:
   ```bash
   cargo build --features async
   cargo test --features async
   cargo build --examples --features async
   ```

This script runs all the same checks that CI will run.

## Caching

All workflows use GitHub Actions cache to speed up builds:
- Caches cargo registry and git dependencies
- Caches build artifacts in the target directory
- Uses separate cache keys for different features to avoid conflicts