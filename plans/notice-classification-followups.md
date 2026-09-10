# Notice classification — follow-ups from PR #810

PR #810 (issues #805, #806) made `messages::classify` the single owner of the
notice precedence chain and derived `Notice::category()` and
`is_informational_code` from it. These are the /simplify and altitude findings
deferred from that PR because each changes observable behaviour or public API
shape beyond the two issues.

## 1. Derive log severity from `NoticeCategory`

`transport::common::log_unrouted_notice` grades an unrouted notice by
`is_warning()` plus one exact code, and `connection::common` logs handshake
notices by `is_warning() || is_system_message()`. Both hand-roll an
approximation of "informational". After #810 an unrouted 2188 logs at `warn`
(it is inside `WARNING_CODE_RANGE`) while 317 and the 10xxx advisories log at
`error` — the same `DataAdvisory` category at two severities depending on the
numeric band.

Fix: one severity mapping on `NoticeCategory` (or a `match` in one place) and
both call sites dispatch on `notice.category()`. Behaviour change: advisories
drop from `error` to `warn`/`info` in logs. Do this before follow-up 2.

## 2. Narrow or rename the band predicates

`Notice::is_warning()` and `is_order_rejection()` answer "is the code inside
this band"; `is_cancellation()` / `is_data_advisory()` answer "is it this
category". The overlap (2188 is a warning by band and an advisory by category;
317 is an order rejection by band and an advisory by category) is documented on
each predicate rather than carried by the name. Options: rename to
`in_warning_band` / `in_order_rejection_band`, or derive them from `category()`.
Deriving today would regress follow-up 1's sites (2188 would log at `error`),
so land 1 first. Public API change — needs a migration note.

## 3. `ORDER_REJECTION_CODE_RANGE` conflates request errors with rejections

300–399 is mostly request errors (309 max depth requests, 316 depth halted,
354 not subscribed, 366 no historical query), all categorised
`OrderRejection`. Harmless for routing (all terminal), misleading for a consumer
attributing failures to orders. The partition test pins `(316, OrderRejection)`
with a comment that the label is the band's. A `RequestError` category, or a
band of 200..=299 plus an explicit list, is the fix; `#[non_exhaustive]` on
`NoticeCategory` makes it non-breaking.

## 4. Model the depth reset in the depth vocabulary

317 rides as `SubscriptionItem::Notice`, which `iter_data()` / `filter_data()`
drop with a `warn!`. A consumer on the ergonomic path keeps a stale book and
merges the rebuild rows into it; the only defence is the doc paragraph on
`MarketDepths`. A `MarketDepths::Reset` variant synthesised when a 317 lands on
a depth subscription survives `iter_data()` and forces an exhaustive-match
decision. Needs a notice-to-data hook in the subscription decoder path that
does not exist yet; design before implementing.

## Skipped /simplify items (recorded, not planned)

- Replace the typed `MarketDepths` MemoryStream tests with the untyped
  `make_request_subscription` + `body("89|42|payload|")` shape. Kept typed: the
  issue asked for the #804 shape and it is the only test driving `MarketDepths`
  through a notice-then-data sequence.
- Doc caveats on `is_warning()` / `is_order_rejection()` / the
  `ORDER_REJECTION_CODE_RANGE` constant still restate the overlap. Kept: those
  are the public surfaces a caller reads; follow-up 2 removes the need.
