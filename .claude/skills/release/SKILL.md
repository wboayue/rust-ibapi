---
name: release
description: Cut an ibapi release from main — version bump PR, dated changelog, annotated tag, draft GitHub release. Stops before publishing; the owner publishes the GitHub release and runs cargo publish.
argument-hint: "[X.Y.Z]"
disable-model-invocation: true
---

# Release ibapi

Releases are cut from `main` only. `v2-stable` / `v1-stable` are out of scope.

Every outward-facing step (merge, tag push, draft release) is **asked before it runs**. The
owner publishes the GitHub release and runs `cargo publish`; this skill never does either.

## 1. Preflight

- On `main`, tree clean, `git pull --ff-only` up to date, CI green on HEAD
  (`gh run list --branch main --limit 3`).
- `PREV=$(git describe --tags --abbrev=0)` — the last release tag.
- Pick the version (or confirm `$ARGUMENTS`) from `## [Unreleased]` in `CHANGELOG.md`:
  - `Removed`, or a breaking `Changed` → major. Ask first: a major starts a new
    `docs/migration-N.0.md`; the previous migration guide is frozen.
  - `Added` → minor.
  - `Fixed` only → patch.

## 2. Changelog audit

Every PR merged since `$PREV` has a bullet or is internal-only.

```bash
git log --oneline "$PREV"..main | grep -oE '#[0-9]+' | sort -u
grep -oE '#[0-9]+' <(sed -n '/^## \[Unreleased\]/,/^## \[/p' CHANGELOG.md) | sort -u
```

Diff the two lists. For each PR with no bullet, read it (`gh pr view N`) and either add a
bullet per [changelog entry](../../../docs/rules/docs/changelog-entry.md) or confirm it is
internal ("would a downstream user notice?"). A squash commit can reference the issue number
rather than the PR — match both. Releases 5.2 (#999) and 5.3 (#1059) each shipped missing one.

## 3. Bump PR

Branch `release-X.Y.Z` off `main`.

1. `Cargo.toml`: `version = "X.Y.Z"`.
2. `CHANGELOG.md`:
   - Rename `## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD` (ISO 8601, today).
   - Add a fresh empty `## [Unreleased]` above it.
   - Link refs at the bottom: `[Unreleased]` → `compare/vX.Y.Z...HEAD`; add
     `[X.Y.Z]: https://github.com/wboayue/rust-ibapi/compare/vPREV...vX.Y.Z`.
3. Install snippets: replace the old `"A.B"` with `"X.Y"` wherever it appears. Find them by
   grep, not a fixed list — `grep -rn '"A\.B"' README.md docs/`. A patch release leaves
   these unchanged.
4. Checks, matching CI (flags from the matrix in `.github/workflows/ci.yml`):
   - `cargo fmt -- --check`
   - `cargo clippy --all-targets <flags> -- -D warnings` for each matrix leg
   - `just test`
   - `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`
   - `cargo publish --dry-run`
5. Commit `Release X.Y.Z`, open the PR (body via `--body-file`), wait for CI green.
6. Ask, then `gh pr merge N --squash` — the bump is one commit.

## 4. Tag

After the merge: `git switch main && git pull --ff-only`, confirm HEAD is the squash commit,
then ask and run `just tag vX.Y.Z` (annotated tag, pushed to origin).

## 5. Draft GitHub release

Write the notes per [release notes](references/release-notes.md) to a scratchpad file, then
ask and run:

```bash
gh release create vX.Y.Z --draft --verify-tag --title "vX.Y.Z" --notes-file <file>
```

## 6. Handoff

Stop. Tell the owner, with the draft URL:

1. Review and publish the draft release on GitHub.
2. `cargo publish` from `main` at `vX.Y.Z`.

Optional, after they confirm: check https://crates.io/crates/ibapi shows X.Y.Z and the
docs.rs build passed.
