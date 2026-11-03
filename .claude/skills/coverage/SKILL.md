---
name: coverage
description: Measure and raise test coverage for a module — toolchain check, baseline against main, a fast per-file llvm-cov loop, reading the numbers without chasing instrumentation noise, and the tactics that close the last gap. Use for "improve coverage of src/X.rs" or a pre-PR coverage check.
argument-hint: "[file or module, e.g. src/contracts/types.rs]"
---

# Coverage

Target: `$ARGUMENTS`. The floor is 90% lines per touched module, and every new `pub` /
`pub(crate)` fn has a test ([coverage floor](../../../docs/rules/testing/coverage-floor.md)).
`$S` is your scratchpad directory.

## 1. Toolchain

Coverage runs on nightly (doc-tests only instrument there); build and test stay on the pinned
stable toolchain. Two version couplings break it:

| Symptom | Cause | Fix |
|---|---|---|
| `rustc …-nightly is not supported by the following packages` | Nightly older than a dependency's MSRV (often after a Dependabot bump) | `rustup update nightly` |
| **0% for every file** while tests pass; or `raw profile version mismatch` | `cargo-llvm-cov` older than nightly | `cargo install cargo-llvm-cov --locked`, then `rm -f target/llvm-cov-target/*.profraw` |

0% everywhere is never a real result. Cross-check with stable
`cargo llvm-cov --lib --all-features` (undercounts doc-tests, but sane).

## 2. Baseline

A percentage means nothing without main's number for the same file. Measure main first, in a
detached worktree so your branch stays untouched:

```bash
git worktree add --detach $S/cov-main origin/main
```

Run §3 there, note the file's numbers, then `git worktree remove $S/cov-main`.
(`src/proto/decoders.rs` once read 85% on a branch, which looked low until main read 41.7%.)

## 3. Per-file loop

`just cover` instruments everything and opens a browser. For one file:

```bash
# instrument + test, no report (~1 min with a warm build cache)
cargo +nightly llvm-cov --all-features --doctests --no-report --quiet

# summary for the target file
F=src/contracts/types.rs
cargo +nightly llvm-cov report --json | jq -r --arg f "$F" '.data[0].files[]
  | select(.filename | endswith($f)) | .summary | to_entries[]
  | select(.key | IN("lines","functions","regions"))
  | "\(.key): \(.value.percent | floor)% (\(.value.covered)/\(.value.count))"'

# which lines are uncovered
cargo +nightly llvm-cov report --summary-only --show-missing-lines | grep "$F"
```

For the line-by-line view, `cargo +nightly llvm-cov report --html --output-dir $S/cov` and
open `$S/cov/html/coverage/<absolute path>.html`; red lines carry `uncovered-line`.

Re-run the first command after each batch of tests; the report commands (~15s) reuse its
profile.

## 4. Read the numbers

- **Track functions and regions, not lines.** `lines.count` is instrumented sites: generics,
  derives and macro expansions move it without the file changing (one module went 241 → 343 →
  245 sites across a refactor while its source stayed ~545 lines).
- **Find gaps in the HTML, not the JSON.** The JSON lists per-monomorphization sub-regions as
  uncovered where the HTML shows the line covered.
- **Noise, not gaps** (on stable or `--lib` runs): `..Default::default()` and doc-comment
  fence lines. Nightly + `--doctests` removes them.
- **Not coverage at all**: a `const fn` called only from `const` initializers reads 0%;
  llvm-cov sees runtime paths only.

## 5. Close the gap

Test behavior through production code first
([exercise production code](../../../docs/rules/testing/exercise-production-code.md)): the
`impl Client` method with a stubbed bus, the decoder with a fixture builder, the error branch
with the frame that triggers it. Then, for what's left:

| Uncovered | Tactic |
|---|---|
| `derive(Serialize, Deserialize)` lines | One serde round-trip per type, through a generic helper (`check_serde_round_trip` in `src/contracts/types_tests.rs`), not a macro |
| `fn new(s: impl Into<String>)` | Call it with both `&str` and `String`: each argument type is a separate function to llvm-cov. Same for any `impl Trait` parameter |
| A `const fn` constructor | One runtime call with an assertion |
| A branch on the current time | A [clock seam](../../../docs/rules/testing/clock-seams.md), not a sleep |
| A sync or async side only | The mirror test on the other side; parity gaps hide here |

Every added test asserts something a reader would care about. A test that only executes a
line to move a number gets cut in review.

## 6. Report

In the PR body, per touched file: main's number → yours, for lines and functions, and the
command that produced them. `target/llvm-cov-target/` grows large; delete it when done if
disk is tight.
