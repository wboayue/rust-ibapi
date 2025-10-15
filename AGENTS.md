# Repository Guidelines

## Project Structure & Module Organization
Core library code resides in `src/`, split by trading domain (`src/market_data`, `src/orders`, `src/client`, etc.) and supported by shared utilities in `src/common`. Public-facing examples live in `examples/`, mirroring both sync and async workflows; use them as templates when adding new samples. Long-form references and migration notes are kept in `docs/` and `MIGRATION.md`. Integration-style fixtures sit under `tests/` with JSON assets in `tests/data`. Tooling configuration (`justfile`, `rustfmt.toml`, `deny.toml`) and coverage artifacts (`tarpaulin-report.html`) are rooted at the repo top level.

## Build, Test, and Development Commands
Use `cargo build --features sync` or `cargo build --features async` to compile the respective transport layer. `cargo test --features sync` and `cargo test --features async` run the targeted suites; `just test` executes both back-to-back. Run `cargo fmt --all` before committing, and `cargo clippy --all-targets --all-features` to catch lint issues across both feature sets. Coverage can be generated with `just cover`, which runs Tarpaulin and refreshes `tarpaulin-report.html`.

## Coding Style & Naming Conventions
Rust code follows the settings in `rustfmt.toml` (4-space indentation, newline-delimited imports). Modules stay in `snake_case`, types and builders in `PascalCase`, and trait implementations in `CamelCase` where idiomatic. Prefer descriptive builder methods (e.g., `Contract::stock(...).on_exchange(...)`) and reuse the domain-specific submodules already established under `src/`. Keep public APIs documented with `///` doc comments; internal helpers only get comments when behavior is non-obvious.

## Testing Guidelines
Unit tests belong alongside modules within `src/`, while broader integration and async validations live in `tests/`. Name new test files by behavior (`*_builder.rs`, `*_async.rs`) and ensure they compile under both `sync` and `async` features when applicable. Use recorded data in `tests/data` instead of live broker calls; refresh fixtures only when protocols change. If a test requires feature-specific setup, guard it with `#[cfg(feature = "sync")]` or `#[cfg(feature = "async")]`.

## Commit & Pull Request Guidelines
Recent history follows Conventional Commits (`feat: ...`, `docs: ...`) with PR references in parentheses; keep that pattern for clear changelog generation. Each PR should summarize the change, list the tested feature flags, and note any updates to examples or docs. Link related issues, attach logs or coverage deltas when relevant, and call out breaking API adjustments in both the description and `MIGRATION.md` when needed.
