# Wire facts

What TWS actually sends and accepts, observed against a live gateway. IBKR docs, the C#
reference client and field names have each been wrong or stale; an entry here is the evidence
behind a decoder, encoder, or rustdoc decision.

| File | Covers |
|---|---|
| [protocol.md](protocol.md) | Connection, request ids, cancels, end markers |
| [orders.md](orders.md) | Order placement, echoes, status, executions, completed orders |
| [contracts.md](contracts.md) | Contract details, option chains |
| [market-data.md](market-data.md) | Generic ticks, option computations, tick types |
| [accounts.md](accounts.md) | Positions, PnL |

## Entry format

```markdown
### <field or behavior>

*Live <YYYY-MM-DD> <session: RTH / off-hours / overnight / weekend>, paper, server <N>; <PRs/issues>.*

- What was sent, what came back (frames, error codes, values).
- What it settles, and what the docs or C# claimed if they differ.
- **Not probed:** what the run could not observe.
```

Rules:

- Every entry carries date, session and server version. A fact from server 220 may not hold
  at 226; re-probe before relying on an old one for a new decision.
- Record what was **not** observed. "Unverified" stops the next reader from treating a gap
  as confirmed.
- A fact that changes how a caller uses an API also goes in that item's rustdoc. This file
  keeps the evidence; rustdoc keeps the instruction.
- Superseded by a later probe: rewrite the entry, keep the old result as one line under it.
- Checks still waiting on market hours live in `plans/live-checks.md` (untracked), not here.

How to run a probe: the `/live-probe` skill ([`.claude/skills/live-probe`](../../.claude/skills/live-probe/SKILL.md)).
