# Code Style Guidelines

## Design Principles

### DRY (Don't Repeat Yourself)
- Extract shared logic to `common/` modules
- Use `request_helpers` for common request patterns
- Prefer traits over code duplication across types
- **When to extract**: 2+ occurrences differing only in parameter values (same logic, types, and control flow)
- **When NOT to extract**: One-time code, or when extraction obscures intent

### SRP (Single Responsibility Principle)
- One encoder/decoder per message type
- Modules own one domain (accounts, orders, market_data)
- Functions do one thing: encode, decode, validate, or orchestrate
- Max 50 lines per function; extract if larger
- Functions with 4+ parameters should use a builder pattern

### Composition
- Combine small, focused components to build complex behavior
- Use traits to define shared behavior across types
- Compose complex types from smaller building blocks:
  ```rust
  // Good: use builder when 4+ params, optional params, or complex construction
  let order = order_builder::limit_order(Action::Buy, 100.0, 150.0)
      .condition(price_condition)
      .build();

  // Bad: monolithic constructor with 4+ params
  let order = Order::new(Action::Buy, 100.0, 150.0, Some(cond), None, None);
  ```
- Prefer `impl Trait` for flexible return types
- Use newtype wrappers for domain constraints

### When Principles Conflict
- Clarity > DRY (if a reader must jump to another file to understand the flow, don't extract)
- SRP > brevity (split even if it adds lines)
- Consistency with existing code > ideal patterns

See [extending-api.md#anti-patterns-to-avoid](extending-api.md#anti-patterns-to-avoid) for code examples of these violations.

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