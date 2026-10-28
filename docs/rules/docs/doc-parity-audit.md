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

"Make the async docs match the sync ones" is the trigger, not the job. Audit three layers and
fix all three in the same PR:

1. **Doc content** — `# Arguments`, `# Examples`, struct-level docs. The asked-for thing.
2. **Parameter naming** — same semantics under different names (`interval_end` vs `end_date`).
3. **Signature shape** — `Option<T>` vs `T`, one method with an `Option` arg vs two methods,
   stringly-typed vs typed args.

The doc gap is what someone noticed; the signature divergence underneath, invisible in rendered
docs, is usually bigger.

Match doc-examples **per method against its cross-feature counterpart**, not against other
examples in the same file. An async example whose imports differ from its async siblings but
agree with its sync twin is correct.

## Before dropping an `Option<T>` to `T`

Check, in order:

1. **The wire** — is `None` representable at all in the encoder?
2. **The C# client** — does `EClient.cs` treat the field as required?
3. **Every caller** — what fraction wrap with `Some(...)`?

If the wire allows `None` but every real caller passes `Some(x)`, drop the `Option`. Tests that
pass `None` on an early-failure path where the argument is never read are not real callsites.

A magic-`None` API split into `foo(arg: T)` + `foo_default(...)` is a **waypoint**: bad shape →
split → fluent builder with a named setter for the optional.

## Precedents

- #573 (issue #210) — doc parity on async historical shipped 9 doc fixes, one signature reshape
  (`Option<WhatToShow>` → `WhatToShow`), one method split (`historical_schedule` →
  `historical_schedules` + `historical_schedules_ending_now`), and one rename (`interval_end` →
  `end_date`). The split later reached step three: both methods are gone, replaced by the
  builder `client.historical_schedules(&contract, duration).fetch()`. The sync test and example
  named `historical_schedules_ending_now` outlive the method.
