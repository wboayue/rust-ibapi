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
precedents: ["#547", "#548", "#581", "#665"]
memory: [feedback_narrowing_transparency_audit, feedback_non_exhaustive_caseby_case]
---

A PR that adds a compile-time restriction to a public type — `#[non_exhaustive]`, `#[must_use]`,
a removed `Default`, a removed or renamed `pub` field — splits in two:

- **PR-A** — migrate every caller to an alternative that is already green today.
- **PR-B** — add the restriction.

Both PRs keep the workspace green, and PR-B stays small enough to review as a policy decision
rather than a diff. Skip the split only when the caller fix is mechanical enough to read as
noise (`let _ = ...` on nine callsites).

Callers include `examples/`, `README.md`, `docs/*.md`, and the
[integration crates](integration-crate-builds.md) — the last are invisible to
`cargo clippy --all-targets`.

Distinct from [floor-ratchet splits](../wire/floor-ratchet-splits.md), which is the same shape
applied specifically to proto floor ratchets.

## `pub` → `pub(crate)` narrowings

"The narrowing is transparent, internal callsites already use crate-local paths" is the claim
that reliably underestimates the work. Crate-local paths say nothing about the *other* public
surfaces that mention the type. Before scoping, grep for it in:

1. **`Error` variant payloads** — `pub enum Error { Foo(T) }`. The variant is public, the payload
   is not; rustc fires `private_interfaces`. Remedy: change the payload to `String` and add an
   `Error::foo(&T) -> Error` constructor.
2. **Public trait method signatures** — same lint, and every impl repeats it. Remedy: the
   sealed-trait pattern, not `#[allow(private_interfaces)]` sprinkled per impl.
3. **Public fn arguments and return types.**
4. **Public struct field types.**

Then expect `dead_code` on top: once the type is `pub(crate)`, any of its methods without an
in-crate caller trips `-D warnings`, even though it compiled fine as a `pub` API.

## Why a restriction can be wrong even when it lands cleanly

`#[non_exhaustive]` is a deliberate choice, not a default — it takes exhaustive matching and
struct-literal construction away from callers permanently. Justify each application; an
attribute added "for headroom" is the one most likely to be reverted.

## Precedents

- #547 → #548 — the canonical split. #547 modernized 15 example sites and a README snippet to
  the typed `Contract` constructors; #548 then added `#[non_exhaustive]` to `Contract`, revived
  `ContractBuilder` as the escape hatch, and added `setter_parity_with_contract_fields`, which
  destructures every `Contract` field so a new field without a setter fails to compile.
- #665 — **removed that `#[non_exhaustive]` again.** Once the fields were strongly typed
  (`Symbol`, `Exchange`, `Currency`, `Option<OptionRight>`), the 2.x mistakes it guarded already
  failed to compile, so it was pure construction friction. The split was still right; the
  restriction was not. The `compile_fail` guard from #548 went with it — the repo's remaining
  one guards a typestate builder, per
  [pin compile_fail codes](../testing/pin-compile-fail-codes.md).
  `setter_parity_with_contract_fields` survives and is the part still earning its keep.
- #581 — narrowing `ResponseMessage` to `pub(crate)` was planned as transparent; it surfaced
  five `private_interfaces` warnings (`TickDecoder::decode`, three impls, and
  `Error::UnexpectedResponse`) plus five `dead_code` warnings, expanding scope mid-flight.
