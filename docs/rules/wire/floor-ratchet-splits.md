---
id: floor-ratchet-splits
title: Ship the protocol floor bump separately from decoder cleanups
cluster: wire
status: historical
triggers:
  - raising the connection protobuf floor
  - IBKR ships a newly proto-gated message family
symbols: [require_protobuf_support, PROTOBUF_REST_MESSAGES_3, server_versions]
related: [fixture-migration, proto-only-decoding]
precedents: ["#527", "#529", "#530", "#531"]
---

**Concluded** at floor `server_versions::PROTOBUF_REST_MESSAGES_3` (213). Kept in case IBKR ships
new gated families.

Ship the floor ratchet — the constant `require_protobuf_support` (`src/connection/common.rs`)
checks against, plus test-fixture version sync — as its own PR, separate from the per-family
decoder text-branch deletions that follow it.

## Why

The bump is mechanical; each cleanup is not. It needs verification against C# `EDecoder.cs` that
the family's proto/text dispatch keys purely on message-id framing, with no `if server_version >=`
guard inside the handler — confirm per family rather than assuming. Mixing the two makes the
mechanical change unreviewable and buries the per-family reasoning.

Skipping several gates at once (203 → 210 crossed six) is safe **only if** every family in the
skipped range already has a proto decoder. Verify against a per-family inventory before bumping.

Cite the floor as `server_versions::PROTOBUF_REST_MESSAGES_3` (213), not a bare number.
`server_versions::PROTOBUF` (201) is a different, lower constant and easy to reach for by mistake.

When the arc concluded, #656 swept `CLAUDE.md`, `README.md` and `docs/` in one pass: rules still
named deleted helpers (`decode_proto_or_text`, `proto_or_text_int`), and README and
`docs/quick-start.md` snippets used APIs replaced several PRs earlier. Each PR's docs grep had
covered only what it touched. A future arc ends the same way; see
[user docs sync](../docs/user-docs-sync.md).

## Precedents

- #527 → #529, #530 → #531 — the bump-then-cleanup pairs.
- #632 — the last ratchet, 210 → 213. The full PR map is in #645.
- #656 — the end-of-arc docs sweep.
