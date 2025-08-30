# Code Style Guidelines

## Comments

- **Keep comments concise and avoid redundancy**. Don't state the obvious.
  - ✅ Good: Complex logic that needs explanation
  - ❌ Bad: `// Initialize connection` right before `Connection::new()`
  - ❌ Bad: `// Set flag to true` right before `flag = true`

- **Inline comments**: Use sparingly, only when the code's intent isn't clear

- **Doc comments**: Required for all public APIs, should explain the "what" and "why", not the "how"

## General Guidelines

1. **No unnecessary comments**: Let the code speak for itself
2. **Meaningful variable names**: Prefer descriptive names over comments
3. **Function length**: Keep functions focused and short
4. **Error handling**: Use proper Result types and error propagation
5. **Avoid unwrap() in production code**: Use `?` or proper error handling

## Formatting

Always run `cargo fmt` before committing code. The project uses default rustfmt settings.

## Linting

Run clippy for both feature flags before committing:
```bash
cargo clippy --features sync -- -D warnings
cargo clippy --features async -- -D warnings
```

## Naming Conventions

- **Types**: PascalCase (e.g., `MessageBus`, `AccountSummary`)
- **Functions/Methods**: snake_case (e.g., `connect`, `server_time`)
- **Constants**: UPPER_SNAKE_CASE (e.g., `MAX_RECONNECT_ATTEMPTS`)
- **Module names**: snake_case (e.g., `market_data`, `order_management`)

## Import Organization

Group imports in this order:
1. Standard library imports
2. External crate imports
3. Internal crate imports
4. Super/self imports

Use blank lines between groups.