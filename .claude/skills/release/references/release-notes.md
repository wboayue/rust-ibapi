# Release notes

Draft from the dated `## [X.Y.Z]` section of `CHANGELOG.md`, not the merge log. A note with
no changelog bullet behind it means some PR skipped its entry — fix the changelog too.

The changelog bullet answers "what changed". The note answers "what do I write now": a
reader decides whether to upgrade based on whether the new call looks like something they
want to type.

## Layout

Copy the shape of v5.2.0 (`gh release view v5.2.0`):

````markdown
5.2 makes `wait_for_fill` find orders this connection hasn't seen, ... and adds TWS API
1051.01 fields. No breaking changes.

## What's New

### `wait_for_fill` finds orders from earlier connections (#993, #1020)
What a user now gets, in a sentence or two.

```rust
// typical usage
```

## Changed

## Bug Fixes

### Order routing with multiple clients (#1021, #1033)
- One bullet per fix in the group.

### Also fixed
- Smaller fixes. Thanks to @user for the contribution (#1029).

See [CHANGELOG.md](https://github.com/wboayue/rust-ibapi/blob/main/CHANGELOG.md#520---2026-10-20) for the full list.

Thanks to @user for the many internal cleanups in this release (#1001–#1005, ...).
````

- Opening paragraph: the release's headline effects in one or two sentences, ending with
  "No breaking changes." or a pointer to the migration guide.
- Omit empty sections; an `## Examples` section is fine when examples changed visibly.
  Order items by significance, most impactful first.
- Headings name the change and its PR(s): `(#123)`, or `(#707, #708)` when several PRs
  landed one feature. Related fixes group under one heading; leftovers go in
  `### Also fixed`.
- The CHANGELOG link anchors the version section: `#XYZ---YYYY-MM-DD` (digits only, three
  hyphens).
- Closing line credits outside work that has no item of its own (internal cleanups, tests).

## Code samples

Every new API (What's New item) gets a short ```rust block of typical usage. v5.3.0 shipped
without them and was flagged.

Compile each sample before publishing: scratch crate in the scratchpad with
`ibapi = { path = "<repo>" }` (plus the feature the sample needs), sample bodies in `main.rs`
or functions taking `&Client`. `cargo check` must pass. Paste the checked code into the notes.

## Attribution

Credit every outside contribution — this is the one thing the notes carry that the changelog
does not. Look up the PR author and the issue reporter:

```bash
gh pr view N --json author,closingIssuesReferences
gh issue view M --json author
```

End the item with `Thanks to @username for the contribution (#N).` (PR author) or
`Thanks to @username for the report (#N).` (issue reporter). Skip the maintainer's own work.
