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

Other agents may share the main checkout and switch its branch under you. Do all release work
in a dedicated worktree, never in the shared checkout; `$S` is your scratchpad directory.

```bash
git fetch origin --tags
WT="$S/release"
git worktree add --detach "$WT" origin/main
cd "$WT"
PREV=$(git describe --tags --abbrev=0)   # last release tag
```

- CI green on `origin/main` HEAD: `gh run list --branch main --limit 3`.
- Pick the version (or confirm `$ARGUMENTS`) from `## [Unreleased]` in `CHANGELOG.md`:
  - `Removed`, or a breaking `Changed` → major. Ask first: a major starts a new
    `docs/migration-N.0.md`; the previous migration guide is frozen.
  - Any `Added`, `Changed` or `Deprecated` → minor.
  - Only `Fixed` / `Security` → patch.

## 2. Changelog audit

Every PR merged since `$PREV` has a bullet or is internal-only. Releases 5.2 (#999) and 5.3
(#1059) each shipped missing one.

List merged PRs from GitHub, not the commit log: squash subjects carry both the issue and the
PR number, and merge commits bring in every branch commit. The list also feeds attribution
in §5.

```bash
SINCE=$(git log -1 --format=%cI "$PREV")
gh pr list --base main --state merged --limit 200 --search "merged:>$SINCE" \
  --json number,title,author,closingIssuesReferences \
  --jq '.[] | "#\(.number)\t\([.closingIssuesReferences[].number | "#\(.)"] | join(","))\t@\(.author.login)\t\(.title)"' \
  > "$S/prs.tsv"
sed -n '/^## \[Unreleased\]/,/^## \[[0-9]/p' CHANGELOG.md | grep -oE '#[0-9]+' | sort -u > "$S/cited.txt"
# PRs whose own number, closing issues, or title refs appear in no bullet
awk -F'\t' 'NR==FNR { cited[$1] = 1; next }
  { refs = $1 "," $2; t = $4
    while (match(t, /#[0-9]+/)) { refs = refs "," substr(t, RSTART, RLENGTH); t = substr(t, RSTART + RLENGTH) }
    n = split(refs, r, ","); hit = 0
    for (i = 1; i <= n; i++) if (r[i] in cited) hit = 1
    if (!hit) print }' "$S/cited.txt" "$S/prs.tsv"
```

For each uncovered PR, read it (`gh pr view N`) and either add a bullet per
[changelog entry](../../../docs/rules/docs/changelog-entry.md) or confirm it is internal
("would a downstream user notice?"). Missing bullets go into the bump PR.

## 3. Bump PR

In `$WT`: `git switch -c release-X.Y.Z`.

1. `Cargo.toml`: `version = "X.Y.Z"`.
2. `CHANGELOG.md`:
   - Rename `## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD` (ISO 8601, today).
   - Add a fresh empty `## [Unreleased]` above it.
   - Link refs at the bottom: `[Unreleased]` → `compare/vX.Y.Z...HEAD`; add
     `[X.Y.Z]: https://github.com/wboayue/rust-ibapi/compare/vPREV...vX.Y.Z`.
3. Install snippets: replace the old `"A.B"` with `"X.Y"` wherever it appears. Find them by
   grep, not a fixed list — `grep -rn '"A\.B"' README.md docs/`. A patch release leaves
   these unchanged.
4. Commit, checking the branch first — `cargo publish --dry-run` below refuses a dirty tree:
   ```bash
   [ "$(git branch --show-current)" = release-X.Y.Z ] && git commit -am "Release X.Y.Z"
   ```
5. Checks. The legs mirror the `flags` matrix in `.github/workflows/ci.yml`; re-read it in case
   it changed. Rustdoc runs per leg too: CI has no doc job, and intra-doc links to
   feature-gated paths break only under some legs.
   ```bash
   cargo fmt -- --check
   for flags in "" "--no-default-features --features sync" "--all-features"; do
     cargo clippy --all-targets $flags -- -D warnings || break
     RUSTDOCFLAGS="-D warnings" cargo doc --no-deps $flags || break
   done
   just test
   cargo publish --dry-run
   ```
   Fixes found here are follow-up commits on the branch; re-run the checks after.
6. Push, open the PR (body via `--body-file`), wait for CI green.
7. Ask, then `gh pr merge N --squash` — the bump is one change.

## 4. Tag

Tag the squash commit by SHA, not whatever HEAD happens to be:

```bash
git fetch origin
SHA=$(gh pr view N --json mergeCommit -q .mergeCommit.oid)
git log -1 --oneline "$SHA"   # expect "Release X.Y.Z (#N)" on origin/main
```

Then ask, and run `git tag -a vX.Y.Z "$SHA" -m "Release X.Y.Z" && git push origin vX.Y.Z`.

## 5. Draft GitHub release

Write the notes per [release notes](references/release-notes.md) to `$S/notes.md`, taking
authors from `$S/prs.tsv`. Then ask and run:

```bash
gh release create vX.Y.Z --draft --verify-tag --title "vX.Y.Z" --notes-file "$S/notes.md"
```

## 6. Handoff

Remove the worktree (`git worktree remove "$WT"`), then stop. Tell the owner, with the draft
URL:

1. Review and publish the draft release on GitHub.
2. `cargo publish` from a clean checkout of `vX.Y.Z`.

Optional, after they confirm: check https://crates.io/crates/ibapi shows X.Y.Z and the
docs.rs build passed.
