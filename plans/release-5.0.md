# 5.0 release backlog

Breaking changes held back from 4.x minors. Land each one in the 5.0 cycle, and
give it a section in `docs/migration-5.0.md`.

## utoipa 5 → 6 (#870)

Dependabot PR #870 bumps `utoipa = { version = "5", ... }` to `"6"` in
`Cargo.toml`. CI is green on every job, including all-features. We use no
`#[schema(...)]` attributes, so utoipa 6 removing the expression form of
`ignore` doesn't touch us. utoipa 6 needs Rust 1.88 or later, and we pin 1.95.

Held because utoipa is a public dependency: public types derive
`utoipa::ToSchema` behind the `utoipa` feature. The v5 and v6 `ToSchema` traits
are distinct, so a downstream crate on utoipa 5 that puts our types in a
`#[derive(OpenApi)]` stops compiling. A `">=5, <7"` range doesn't help, because
cargo can still pick 6 for us and 5 for the user.

To do:

- Merge #870, or redo the bump if it has gone stale.
- Re-check the hand-written `PartialSchema`/`ToSchema` impls against utoipa 6.
  These are the wire-string enums such as `OrderStatusKind` and `TimeInForce`
  (see `docs/rules/wire/enum-typing.md`).
- Migration note: users of the `utoipa` feature must move to utoipa 6.
