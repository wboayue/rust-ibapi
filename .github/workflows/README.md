# CI/CD Workflows

This directory contains the GitHub Actions workflows for the rust-ibapi project.

## Workflows

### ci.yml
The main CI workflow that runs on every push and pull request to the main branch. It includes:

#### Test Job
- **Matrix**: One leg per feature configuration: `async` (default), `sync` (sync-only), `all-features`
- **Steps**:
  - Build the library with appropriate features
  - Run all tests
  - Build all examples to ensure they compile

#### Clippy Job
- **Matrix**: Runs clippy for each of the three feature configurations
- **Steps**:
  - Runs clippy with warnings as errors (`-D warnings`)
  - Checks all targets including tests and examples

#### Format Job
- Runs once (formatting is feature-independent)
- Checks that all code is properly formatted with `cargo fmt`

#### Documentation Job
- **Matrix**: Builds docs for each of the three feature configurations
- Ensures documentation compiles without errors

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

This script runs all the same checks that CI will run.

## Caching

All workflows use GitHub Actions cache to speed up builds:
- Caches cargo registry and git dependencies
- Caches build artifacts in the target directory
- Uses separate cache keys for different features to avoid conflicts