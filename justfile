
# Compile proto files to Rust (output: src/proto/protobuf.rs)
proto:
    cargo run -p proto-gen

# Generate and open coverage report using cargo-llvm-cov.
# Uses nightly + --doctests so doc-test examples contribute and to avoid
# stable's phantom instrumentation on `..Default::default()` and doc-fence
# lines (rustdoc's --persist-doctests is nightly-only).
cover:
    cargo +nightly llvm-cov --all-features --doctests --html --open

# Validate the docs/rules/ knowledge graph and its CLAUDE.md index
rules-check:
    ./tools/check-rules-graph.sh

# Tags repo with specified version
tag VERSION:
    echo "Tagging repo with version {{VERSION}}"
    git tag {{VERSION}} -m "Version {{VERSION}}"
    git push origin {{VERSION}}

# Lists all available versions
versions:
    @git tag

# Run tests for both sync and async features
# One leg per feature configuration. `--features sync` would NOT be the sync-only
# build: default = ["async"] and cargo features are additive, so it means sync AND
# async. See docs/rules/parity/feature-matrix.md.
test:
    @echo "Running async tests (default features)..."
    cargo test
    @echo ""
    @echo "Running sync-only tests..."
    cargo test --no-default-features --features sync
    @echo ""
    @echo "Running all-features tests (sync + async + utoipa)..."
    cargo test --all-features

# Run sync integration tests (requires running gateway)
integration-sync:
    cargo test -p ibapi-integration-sync

# Run async integration tests (requires running gateway)
integration-async:
    cargo test -p ibapi-integration-async

# Run all integration tests (requires running gateway)
integration: integration-sync integration-async
