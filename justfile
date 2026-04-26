
# Compile proto files to Rust (output: src/proto/protobuf.rs)
proto:
    cargo run -p proto-gen

# Generate and open coverage report using cargo-llvm-cov
cover:
    cargo llvm-cov --all-features --html --open

# Tags repo with specified version
tag VERSION:
    echo "Tagging repo with version {{VERSION}}"
    git tag {{VERSION}} -m "Version {{VERSION}}"
    git push origin {{VERSION}}

# Lists all available versions
versions:
    @git tag

# Run tests for both sync and async features
test:
    @echo "Running sync tests..."
    cargo test --features sync
    @echo ""
    @echo "Running async tests..."
    cargo test --features async

# Run sync integration tests (requires running gateway)
integration-sync:
    cargo test -p ibapi-integration-sync

# Run async integration tests (requires running gateway)
integration-async:
    cargo test -p ibapi-integration-async

# Run all integration tests (requires running gateway)
integration: integration-sync integration-async
