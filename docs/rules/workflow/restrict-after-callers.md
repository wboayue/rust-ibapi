---
id: restrict-after-callers
title: Modernize callers first, add the restriction second
cluster: workflow
status: active
triggers:
  - adding #[non_exhaustive], #[must_use], or removing a Default impl
  - removing or renaming a pub field
  - narrowing a pub item to pub(crate)
  - a plan says a narrowing is "transparent" to callers
symbols: [non_exhaustive, must_use, private_interfaces, pub(crate)]
related: [integration-crate-builds, modernize-touched-modules, floor-ratchet-splits]
precedents: ["#547", "#548", "#581", "#665", "#1081"]
memory: [feedback_narrowing_transparency_audit, feedback_non_exhaustive_caseby_case]
---

A PR that adds a compile-time restriction to a public type — `#[non_exhaustive]`, `#[must_use]`,
a removed `Default`, a removed or renamed `pub` field — splits in two:

- **PR-A** — migrate every caller to an alternative that is already green today.
- **PR-B** — add the restriction.

Both keep the workspace green, and PR-B stays small enough to review as a policy decision.
Skip the split only when the caller fix is mechanical noise (`let _ = ...` on nine callsites).

Callers include `examples/`, `README.md`, `docs/*.md`, and the
[integration crates](integration-crate-builds.md), which `cargo clippy --all-targets` never sees.
[Floor-ratchet splits](../wire/floor-ratchet-splits.md) is the same shape applied to proto floor
ratchets.

## `pub` → `pub(crate)` narrowings

"Transparent, internal callsites already use crate-local paths" reliably underestimates the
work: it says nothing about the *other* public surfaces that mention the type. Before scoping,
grep for it in:

1. **`Error` variant payloads** — the variant is public, the payload is not; rustc fires
   `private_interfaces`. Remedy: payload becomes `String`, plus a constructor taking `&T`
   (`Error::unexpected_response(&ResponseMessage)`).
2. **Public trait method signatures** — same lint, repeated on every impl. Seal the trait if
   you can; `TickDecoder` instead carries `#[allow(private_interfaces)]` on the trait and each
   impl (#581).
3. **Public fn arguments and return types.**
4. **Public struct field types.**

Then expect `dead_code`: once the type is `pub(crate)`, any method without an in-crate caller
trips `-D warnings`.

## A restriction can be wrong even when it lands cleanly

`#[non_exhaustive]` takes exhaustive matching and struct-literal construction away from callers
permanently. Justify each application; one added "for headroom" is the likeliest to be reverted.

## Precedents

- #547 → #548 — the canonical split. #547 moved 15 example sites and a README snippet to the
  typed `Contract` constructors; #548 added `#[non_exhaustive]` to `Contract`, revived
  `ContractBuilder` as the escape hatch, and added `setter_parity_with_contract_fields`, which
  destructures every `Contract` field so a field without a setter fails to compile.
- #665 — **removed that `#[non_exhaustive]`.** Once fields were strongly typed (`Symbol`,
  `Exchange`, `Currency`, `Option<OptionRight>`), the mistakes it guarded already failed to
  compile. The split was right; the restriction was not. Its `compile_fail` guard went with it (the two
  remaining guard a typestate builder and an id type, #845, per
  [pin compile_fail codes](../testing/pin-compile-fail-codes.md));
  `setter_parity_with_contract_fields` survives.
- #581 — narrowing `ResponseMessage` to `pub(crate)`, planned as transparent, surfaced five
  `private_interfaces` warnings (`TickDecoder::decode`, three impls, `Error::UnexpectedResponse`)
  plus five `dead_code` warnings mid-flight.
- #1081 — removed `#[non_exhaustive]` from `ExecutionFilterSide`, `OptionRight`, `LegAction`:
  closed TWS vocabularies; the one stated reason (on `ExecutionFilterSide`) was headroom.
