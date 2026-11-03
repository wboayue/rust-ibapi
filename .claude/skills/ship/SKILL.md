---
name: ship
description: Take a finished change from local gates to a merged PR — pick the gates the diff needs, self-review, open the PR, get CI to run, choose the merge strategy, handle stacks and fork PRs, clean up. Asks before merging.
argument-hint: "[PR number to merge, or empty to ship the current branch]"
---

# Ship a change

From "the code is done" to merged on `main`. Releases are a separate flow: `/release`.

**Outward-facing steps are asked first:** push, `gh pr create`, and merge each need the
user's go-ahead unless they already gave it for this change ("open a PR", "merge it").
Approval for one PR doesn't carry to the next.

`$S` is your scratchpad directory.

## 1. Branch

- Never commit to `main`. Substantive changes go branch → push → PR; a direct push can't be
  undone (force-push to `main` is blocked even for admins). If it happens anyway: `git revert`
  on `main`, re-land via PR.
- Work in a worktree on its own branch. Other agents may share the main checkout and switch
  its branch under you.

## 2. Gates

Always run the full list in [pre-PR checks](../../../docs/rules/workflow/pre-pr-checks.md),
on the pinned toolchain. Then add what the diff touches:

```bash
git diff --name-only origin/main...HEAD
```

| Diff touches | Also |
|---|---|
| `src/proto/`, decoders, encoders, `subscriptions/`, any public signature | [integration crate builds](../../../docs/rules/workflow/integration-crate-builds.md) (no CI job builds them) |
| `AGENTS.md`, `docs/rules/`, `docs/wire-facts/`, `.claude/skills/` | `just rules-check` |
| A new `pub` / `pub(crate)` fn | test + [coverage floor](../../../docs/rules/testing/coverage-floor.md); `pub fn` also needs a [doc example](../../../docs/rules/docs/public-api-examples.md) |
| Behavior a user would notice | [changelog entry](../../../docs/rules/docs/changelog-entry.md) under `## [Unreleased]`, same PR |
| Removed or renamed public item | [user docs sync](../../../docs/rules/docs/user-docs-sync.md); `grep -rn <old name> README.md docs/ examples/` |
| Deleted or renamed any symbol | `grep -rn <symbol> docs/rules/ docs/wire-facts/ .claude/skills/` |
| A breaking change | stop and ask: it queues for the next major |

Docs-only diffs (`*.md`, skills) skip the cargo gates but not `rules-check`.

## 3. Self-review

Read the full diff as a reviewer would (`/code-review` or the `review-code` skill for a
second read). At minimum:

- **Duplication**: no helper re-implementing one that exists (`ToField`, `some_str`, shared
  test helpers); no logic written twice across sync/async.
- **Tests**: every new `pub` / `pub(crate)` fn has one that exercises production code.
- **Consistency**: the touched module matches its siblings' layout and idiom.
- **Leftovers**: no PR-narrative comments ("added in this PR"), no `plans/` paths, no
  `file.rs:NNN` citations, no temp `#[ignore]` probe tests, no unused imports.

Fix what you find before opening the PR, not in review rounds.

## 4. Commit and PR

- Messages: concise, technical, say what changed and why. No AI attribution lines or footers.
- Anything longer than one line, or containing quotes/backticks, goes through a file:
  `git commit -F $S/msg.txt`, `gh pr create --body-file $S/pr.md`. Inline heredocs inside
  chained commands have failed and taken the commit down with them.
- PR body: why first, then what. Cite issues as `#N`; `Closes #N` for the one it resolves.
  Say what was verified and how (which gates, a live probe), so claims are checkable.
- Title scoped like the log: `<area>: <change>`.

## 5. CI

```bash
gh pr checks <N> --watch --interval 30
```

- CI runs only on PRs whose base is `main` (or `v2-stable`). A stacked child gets **no CI**
  until retargeted; rely on local gates meanwhile.
- **Retargeting does not start CI.** After `gh pr edit <N> --base main`, push something:
  `git merge origin/main && git push` on the branch.
- A red leg: reproduce locally with that leg's exact flags before pushing a fix.

## 6. Merge

Ask first unless told to merge. State the strategy and why. Take the first row that matches:

| Branch holds | Command |
|---|---|
| A child PR stacked on it | `gh pr merge <N> --merge` (see Stacks) |
| One commit | `gh pr merge <N> --squash` |
| Several commits forming one change, each builds | `gh pr merge <N> --merge` |
| Several independent small commits, nothing stacked on top | `gh pr merge <N> --rebase` |
| Fixup / review-response / merge-from-main commits, or a fork PR | `gh pr merge <N> --squash` |
| …and an abandoned approach among the commits | `--squash --body-file $S/body.md` describing only the final design (the default body concatenates every commit) |

**Stacks**, in this order:
1. `gh pr edit <child> --base main` while the child is open. Merging the parent first deletes
   its branch, which can close the child for good: it then can't be reopened or retargeted
   (#723 was lost this way and reopened as #724).
2. Merge the parent with `--merge` (a squashed parent leaves the child with duplicate commits).
3. Push to the child to start its CI (§5), wait for green, merge it.

**Fork PRs** with conflicts: merge `main` into the contributor's branch, never rebase. A
rebase needs a force-push over their history. Check `gh pr view <N> --json
isCrossRepository,maintainerCanModify` first.

**Verify** with `gh pr view <N> --json state,mergedAt`. When `gh pr merge` errors on local
cleanup (`'main' is already used by worktree`), the server-side merge has usually succeeded;
check before re-running.

## 7. Clean up

The repo deletes merged branches on GitHub (`delete_branch_on_merge`), so don't pass
`--delete-branch`.

```bash
git worktree remove <path>
git branch -D <branch>
git fetch -p
```

Follow-ups deferred during review go in GitHub issues, cited as `#N` in the PR
([plans and issues](../../../docs/rules/workflow/plans-and-issues.md)).
