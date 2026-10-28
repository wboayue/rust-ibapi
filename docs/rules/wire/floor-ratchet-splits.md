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
memory: [project_protobuf_only, feedback_end_of_arc_doc_audit]
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

## Precedents

- #527 → #529, #530 → #531 — the bump-then-cleanup pairs.
