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

**Concluded** at floor `server_versions::PROTOBUF_REST_MESSAGES_3` (213). Kept in case IBKR
ships new gated families.

Ship the floor ratchet — the `require_protobuf_support` constant bump plus test-fixture
version sync — as its own PR, separate from the per-family decoder text-branch deletions
that follow it.

## Why

The bump is mechanical. Each cleanup is not: it needs verification against the C#
`EDecoder.cs` that the family's proto/text dispatch isn't server-version-gated *within* the
case. Dispatch keys purely on the 4-byte message-id framing, with no `if server_version >=`
guards inside the handler — confirm that per family rather than assuming it.

Mixing the two makes the mechanical change unreviewable and buries the per-family reasoning.

## Multi-gate ratchets

Skipping several gates at once (203 → 210 crosses six) is safe **only if** every family in
the skipped range already has a proto decoder in place. Verify against a per-family inventory
before bumping.

## Naming

Cite the floor as `server_versions::PROTOBUF_REST_MESSAGES_3` (213), not as a bare number.
`server_versions::PROTOBUF` is a different, lower constant (201) and is easy to reach for by
mistake.

## Precedents

- #527 → #529, #530 → #531 — the bump-then-cleanup pairs.
