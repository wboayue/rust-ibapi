---
id: doc-parity-audit
title: A doc-parity request is a signature audit
cluster: docs
status: active
triggers:
  - asked to make async docs match sync, or the reverse
  - comparing a domain's sync.rs and async.rs surfaces
  - about to change Option<T> to T on a public argument
symbols: [Option, Arguments, Examples]
related: [public-api-examples, dual-feature-types, modernize-touched-modules, param-budget]
precedents: ["#573"]
memory: [feedback_doc_parity_signature_audit, feedback_remove_option_when_all_callers_wrap, feedback_per_method_sync_async_doc_pairing, feedback_magic_none_split_to_builder]
---

"Make the async docs match the sync ones" is the trigger, not the job. Audit three layers
before opening the PR and fix all three in the same PR:

1. **Doc content** — `# Arguments`, `# Examples`, struct-level docs. The asked-for thing.
2. **Parameter naming** — same semantics under different names (`interval_end` vs `end_date`).
3. **Signature shape** — `Option<T>` vs `T`, one method with an `Option` arg vs two methods,
   stringly-typed vs typed args.

The doc gap is what someone noticed. The signature divergence underneath it is usually the
bigger fish, and it is invisible from the rendered docs.

Match doc-examples **per method against its cross-feature counterpart**, not against the other
examples in the same file. An async example whose imports differ from its async siblings but
agree with its sync twin is correct; a reviewer flagging that as intra-file inconsistency is
missing the pairing axis.

## Before dropping an `Option<T>` to `T`

Three sources, in order:

1. **The wire** — is `None` representable at all in the encoder?
2. **The C# client** — does `EClient.cs` document the field as required?
3. **Every caller** — what fraction wrap with `Some(...)`?

If the wire allows `None` but every real caller passes `Some(x)`, drop the `Option`. Tests that
pass `None` out of laziness — exercising an early-failure path where the argument never gets
read — are not real `None` callsites.

When a magic-`None` API gets split into `foo(arg: T)` + `foo_default(...)`, treat the split as
a **waypoint**: bad shape → split → fluent builder with a named setter for the optional. The
split is not the answer, it is the step that makes the builder obvious.

## Precedents

- #573 (issue #210) — asked for doc parity on async historical. Shipped 9 doc fixes, one
  signature reshape (`Option<WhatToShow>` → `WhatToShow`), one method split
  (`historical_schedule` → `historical_schedules` + `historical_schedules_ending_now`), and one
  parameter rename (`interval_end` → `end_date`). The doc gap was the smallest part.

  **The split has since completed its third step.** Both methods are gone; the surface is now
  `client.historical_schedules(&contract, duration).fetch()`, a builder — which is what the
  waypoint rule predicts. Test and example *names* still say `..._ending_now`; the method does
  not exist. Read the split as the middle of a story, not as the destination.
