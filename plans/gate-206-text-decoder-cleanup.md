# Gate-206 text-decoder cleanup (`decode_tick_news` + `decode_option_computation`)

Closes the last two leftovers from the floor-210 sweep listed in
`plans/legacy-text-protocol-cleanup.md` (§"Decoders whose text branch is now
unreachable at floor 210"). Both message types gate at `PROTOBUF_MARKET_DATA`
(206), but their decoders live outside the realtime market_data domain — they
were skipped during PR #543 because the audit was scoped to
`market_data/realtime/`.

Floor is currently `PROTOBUF_SCAN_DATA` (210); both gates are < 210, so the
text branches are already unreachable in production.

## Per-decoder gate analysis

Sourced from `/Users/wboayue/projects/tws-api/source/csharpclient/client/Constants.cs`
(`PROTOBUF_MSG_IDS`).

| Decoder                              | Location                                            | Incoming msg id                | Gate                  |
|--------------------------------------|-----------------------------------------------------|--------------------------------|----------------------:|
| `decode_tick_news`                   | `src/news/common/decoders.rs:56`                    | `TickNews` (84)                | 206 (market_data)     |
| `decode_option_computation`          | `src/contracts/common/decoders/mod.rs:30`           | `TickOptionComputation` (21)   | 206 (market_data)     |

## C# verification

`EDecoder.cs` dispatches both cases purely on 4-byte msg-id framing — no
`if serverVersion >=` guards inside either case body. Safe to delete the text
branches without per-field version checks.

`TickOptionComputation` already has a proto decoder used by realtime market_data
(`decode_tick_option_computation_proto` in `src/market_data/realtime/common/decoders/mod.rs:287`).
`TickNews` does **not** yet have a proto decoder — this PR (or its sibling)
adds one.

## Split

Two small PRs, independent. Either order:

- **PR A**: `decode_tick_news` → proto-only (news domain). Adds a proto
  decoder; otherwise mirrors the news cleanup shape from PR #534.
- **PR B**: `decode_option_computation` → delete + redirect to realtime's
  existing proto decoder (contracts domain). No new proto decoder needed.

PR A touches `src/news/`, `src/testdata/builders/news.rs`, news tests.
PR B touches `src/contracts/`, `src/testdata/builders/contracts.rs`, contracts
tests. Zero overlap.

---

## PR A — `decode_tick_news` (news domain)

### 1. Add the proto decoder function

`proto::TickNews` is already generated (`src/proto/protobuf.rs:1684`); only the
decoder function needs adding. In `src/news/common/decoders.rs`, alongside
`decode_news_article_proto` / `decode_historical_news_proto`:

```rust
pub(crate) fn decode_tick_news_proto(bytes: &[u8]) -> Result<NewsArticle, Error> {
    let p = crate::proto::TickNews::decode(bytes)?;

    let millis = p.timestamp.unwrap_or_default();
    let time = OffsetDateTime::from_unix_timestamp(millis / 1000)
        .map_err(|e| Error::parse_field(&millis.to_string(), e.to_string()))?;

    Ok(NewsArticle {
        time,
        provider_code: p.provider_code.unwrap_or_default(),
        article_id: p.article_id.unwrap_or_default(),
        headline: p.headline.unwrap_or_default(),
        extra_data: p.extra_data.unwrap_or_default(),
    })
}
```

Note: `TickNews.timestamp` is `int64` *milliseconds* on the wire. Verified
against C# reference: `EWrapper.cs:718` declares `long timeStamp` and the
sample app at `samples/CSharp/IBSampleApp/ui/NewsManager.cs:63` calls
`Utils.UnixMillisecondsToString(tickNewsMessage.TimeStamp, ...)`. The text
decoder's `parse_unix_timestamp` (which divides by 1000) was consistent with
this.

### 2. Convert the dispatcher wrapper

Replace the text decoder body with a proto-only wrapper (matches the shape
of `decode_news_bulletin` etc.):

```rust
// before
pub(in crate::news) fn decode_tick_news(mut message: ResponseMessage) -> Result<NewsArticle, Error> {
    message.skip(); // message type
    message.skip(); // request id
    let time = message.next_string()?;
    let time = parse_unix_timestamp(&time)?;
    Ok(NewsArticle { time, provider_code: message.next_string()?, ... })
}

// after
pub(in crate::news) fn decode_tick_news(message: &ResponseMessage) -> Result<NewsArticle, Error> {
    decode_tick_news_proto(message.require_proto()?)
}
```

Receiver flips `mut ResponseMessage` → `&ResponseMessage` (matches the rest of
the file).

Delete the comment block at lines 16-18 about tick_news staying text-framed
(no longer accurate) and the now-unused `parse_unix_timestamp` helper at
lines 72-79 (and its `decoders_tests.rs::test_parse_unix_timestamp` +
`test_parse_unix_timestamp_invalid` siblings — both untraced after the helper
goes).

### 3. Caller signature update

Single callsite in `src/news/common/stream_decoders.rs:37`:

```rust
// before
IncomingMessages::TickNews => decoders::decode_tick_news(message.clone()),

// after
IncomingMessages::TickNews => decoders::decode_tick_news(message),
```

The `.clone()` was needed because the text decoder consumed the message; the
proto wrapper takes `&ResponseMessage`. Drop the clone.

### 4. Testdata response builder

Add to `src/testdata/builders/news.rs`, after `NewsArticleResponse`:

```rust
/// Builder for `TickNews` (msg 84) responses.
#[derive(Clone, Debug)]
pub struct TickNewsResponse {
    pub request_id: i32,
    pub timestamp_millis: i64,
    pub provider_code: String,
    pub article_id: String,
    pub headline: String,
    pub extra_data: String,
}

impl Default for TickNewsResponse { /* TEST_REQ_ID_FIRST + empty strings */ }

impl TickNewsResponse {
    pub fn request_id(mut self, v: i32) -> Self { ... }
    pub fn timestamp_millis(mut self, v: i64) -> Self { ... }
    pub fn provider_code(mut self, v: impl Into<String>) -> Self { ... }
    pub fn article_id(mut self, v: impl Into<String>) -> Self { ... }
    pub fn headline(mut self, v: impl Into<String>) -> Self { ... }
    pub fn extra_data(mut self, v: impl Into<String>) -> Self { ... }
}

impl ResponseProtoEncoder for TickNewsResponse {
    type Proto = proto::TickNews;

    fn to_proto(&self) -> Self::Proto {
        proto::TickNews {
            req_id: Some(self.request_id),
            timestamp: Some(self.timestamp_millis),
            provider_code: some_str(&self.provider_code),
            article_id: some_str(&self.article_id),
            headline: some_str(&self.headline),
            extra_data: some_str(&self.extra_data),
        }
    }
}

pub fn tick_news() -> TickNewsResponse { TickNewsResponse::default() }
```

### 5. Test-fixture migration

Convert the `NEWS_ARTICLE_RESPONSE` text fixture and its callers to proto:

| File                          | Lines using `NEWS_ARTICLE_RESPONSE`                  |
|-------------------------------|------------------------------------------------------|
| `src/news/sync_tests.rs`      | const at L17; consumed at L150 (contract_news), L180 (broad_tape_news) |
| `src/news/async_tests.rs`     | const at L20; consumed at L158 (contract_news), L193 (broad_tape_news), L250 (contract_news_cancellation) |

Replace the const with inline `proto_response(...)` calls in each test (matches
how `test_news_providers` / `test_news_bulletins` are already shaped):

```rust
let message_bus = Arc::new(MessageBusStub::with_ordered_responses(vec![proto_response(
    IncomingMessages::TickNews,
    tick_news()
        .request_id(9000)
        .timestamp_millis(1672531200_000)  // 2023-01-01 00:00:00 UTC
        .provider_code("BZ")
        .article_id("BZ$123")
        .headline("Breaking news headline")
        .extra_data("TSLA:123")
        .encode_proto(),
)]));
```

Watch the timestamp: the old text fixture says `1672531200` (10 digits =
seconds), but `parse_unix_timestamp` divided by 1000 treating it as millis →
parses to 1969-01-20, not 2023-01-01. No test asserted the parsed time, which
is why this has gone unnoticed. The proto field is milliseconds (verified
above), so write `1672531200_000` for a clean Jan 1 2023.

Add `decode_tick_news_rejects_text_framing` to `decoders_tests.rs` mirroring
the existing `_rejects_text_framing` tests in that file (lines 109-134).

Add `test_decode_tick_news_proto` happy-path to `decoders_tests.rs` mirroring
`test_decode_news_article_proto` / `test_decode_historical_news_proto`.

### 6. Sweep

```bash
cargo test                                         # default (async)
cargo test --features sync
cargo test --all-features
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features sync -- -D warnings
cargo clippy --all-features -- -D warnings
cargo fmt
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo build -p ibapi-integration-sync  --tests
cargo build -p ibapi-integration-async --tests
```

### 7. Tracker update

In `plans/legacy-text-protocol-cleanup.md`:

- Move `decode_tick_news` from "Decoders whose text branch is now unreachable
  at floor 210" → the "Floor 210 deletions (shipped)" list, with PR number
- Update per-domain table — `news/common/decoders.rs` goes from "1 text-decoder,
  4 proto-decoders, 0 dual-format" to "0 / 5 / 0"
- Note that `parse_unix_timestamp` helper was also deleted

---

## PR B — `decode_option_computation` (contracts domain)

### Decision: delete, don't rewrite

`decode_tick_option_computation_proto` already exists in
`src/market_data/realtime/common/decoders/mod.rs:287` and is fully tested at
`src/market_data/realtime/common/decoders/tests.rs:499-538`. Both contracts
and realtime decode the **same** `TickOptionComputation` (msg 21) proto into
the **same** `OptionComputation` struct. Reusing the realtime decoder is rule
14 territory (narrow cross-domain re-export over duplication).

### 1. Delete the text decoder

In `src/contracts/common/decoders/mod.rs`:

- Delete `decode_option_computation` (lines 30-66)
- Delete the `next_optional_double` helper (lines 68-75) — only the deleted
  body uses it; grep confirms no other callsite in contracts/
- Delete the `TickType` import on line 1 if no other line in the file uses it
  (it's only referenced inside the deleted body)

In `src/contracts/common/decoders/tests.rs`:

- Delete `test_decode_option_computation` (lines 18-36)
- Delete `test_next_optional_double` (lines 4-17)
- Drop the now-unused `use crate::contracts::tick_types::TickType` (line 2)

### 2. Redirect the dispatcher

In `src/contracts/common/stream_decoders.rs:19`:

```rust
// before
IncomingMessages::TickOptionComputation => Ok(decoders::decode_option_computation(context.server_version, message)?),

// after
IncomingMessages::TickOptionComputation => {
    crate::market_data::realtime::common::decoders::decode_tick_option_computation(message)
}
```

`decode_tick_option_computation` is already `pub(crate)` in realtime, so
this works without changing its visibility. The `context.server_version` arg
becomes unused at this callsite — audit `DecoderContext` usage in the rest of
`OptionComputation::decode` to confirm the `context` parameter is still
load-bearing (the `cancel_message` impl uses it for `request_type` dispatch,
so it stays).

Alternative if the cross-domain reach feels wrong: add a narrow re-export
`pub(crate) use crate::market_data::realtime::common::decoders::decode_tick_option_computation;`
at the top of `src/contracts/common/decoders/mod.rs`, then call
`decoders::decode_tick_option_computation(message)` from stream_decoders.rs.
Either form is fine; the re-export keeps callsites in stream_decoders looking
local. Pick whichever is more readable in the diff.

### 3. Migrate test fixtures

**Three** fixture groups in `src/contracts/common/test_tables.rs`. The text
decoder's slot layout depends on `server_version`: at `>= PRICE_BASED_VOLATILITY`
(156), slot 4 reads `tick_attribute`; below, it doesn't. Each fixture group
runs at a different `server_version`, so each has a different slot layout
that must be traced explicitly when generating the proto builder calls.

After migration, the `server_version` becomes irrelevant to the decoder (proto
path ignores it). The fixture's slot layout is fixed by `proto::TickOptionComputation`
tag ordering. Step (d) below covers an optional server_version bump for
hygiene.

#### (a) `option_calculation_test_cases()` — lines 497-542

2 cases, runs at `server_versions::REQ_CALC_OPTION_PRICE` (50) — **below**
`PRICE_BASED_VOLATILITY`, so the text decoder reads `message_version` and
skips `tick_attribute`. Field-by-field trace for the first case
`"21\06\09000\013\00.3\00.42\07.85\0-0.03\00.65\0-0.002\00.98\06.87\0145.0\07.85\0"`:

| Wire slot | Value     | Field consumed                 |
|-----------|-----------|--------------------------------|
| 1         | `21`      | `message.skip()` (msg type)    |
| 2         | `6`       | `message_version` (next_int)   |
| 3         | `9000`    | `message.skip()` (req id)      |
| 4         | `13`      | `tick_type` (= ModelOption)    |
| 5         | `0.3`     | `implied_vol`                  |
| 6         | `0.42`    | `delta`                        |
| 7         | `7.85`    | `option_price` (mv≥6 \|\| ModelOption) |
| 8         | `-0.03`   | `pv_dividend`                  |
| 9         | `0.65`    | `gamma` (mv≥6)                 |
| 10        | `-0.002`  | `vega`                         |
| 11        | `0.98`    | `theta`                        |
| 12        | `6.87`    | `underlying_price`             |
| 13–14     | `145.0`, `7.85` | unread                   |

Assertions check `option_price == 7.85`, `delta == 0.42`. The second case
`"21\06\09000\013\00.25\00.32\05.0\0-0.02\00.45\0-0.001\00.25\04.55\0145.0\05.0\0"`
walks the same slot mapping; assertions check `option_price == 5.0`,
`delta == 0.32`.

Change the struct field `response_message: String` → `ordered_responses: Vec<ResponseMessage>`
and build:

```rust
proto_response(
    IncomingMessages::TickOptionComputation,
    tick_option_computation()
        .request_id(9000)
        .tick_type(13)
        .implied_volatility(0.3)
        .delta(0.42)
        .option_price(7.85)
        .present_value_dividend(-0.03)
        .gamma(0.65)
        .vega(-0.002)
        .theta(0.98)
        .underlying_price(6.87)
        .encode_proto(),
)
```

#### (b) `client_method_test_cases()` — lines 763+

Currently uses `response_messages: Vec<String>` (plural). Runs at
`REQ_CALC_OPTION_PRICE` (50) for one variant, `REQ_CALC_IMPLIED_VOLAT` (49)
for the other — both below `PRICE_BASED_VOLATILITY`, so same slot layout as
(a). Trace the existing text fixture
`"21|6|9000|13|0.25|0.42|85.5|-0.03|0.65|-0.002|0.98|6.87|155.0|85.5|"`:

| Wire slot | Value     | Field consumed                 |
|-----------|-----------|--------------------------------|
| 1         | `21`      | msg type                       |
| 2         | `6`       | `message_version`              |
| 3         | `9000`    | req id                         |
| 4         | `13`      | `tick_type`                    |
| 5         | `0.25`    | `implied_vol`                  |
| 6         | `0.42`    | `delta`                        |
| 7         | `85.5`    | `option_price`                 |
| 8         | `-0.03`   | `pv_dividend`                  |
| 9         | `0.65`    | `gamma`                        |
| 10        | `-0.002`  | `vega`                         |
| 11        | `0.98`    | `theta`                        |
| 12        | `6.87`    | `underlying_price`             |
| 13–14     | `155.0`, `85.5` | unread                   |

Assertions check `option_price == Some(85.5)`, `implied_volatility == Some(0.25)`.
Map onto proto builder identically.

Change `response_messages: Vec<String>` → `ordered_responses: Vec<ResponseMessage>`
and convert each fixture analogously.

#### (c) `stream_decoder_test_cases()` — lines 660-697

**Missed in the original plan.** Has one option-computation fixture at
line 664 — `text_response("21|6|9000|13|0.3|0.35|5.25|-0.025|0.55|-0.0015|0.3|4.75|140.0|5.25|")`,
consumed by `sync/tests.rs:194` and `async/tests.rs:209`. The test calls
`OptionComputation::decode(&DecoderContext::new(server_versions::SIZE_RULES, None), ...)`
directly. `SIZE_RULES = 164 ≥ PRICE_BASED_VOLATILITY (156)`, so the text
decoder sets `message_version = i32::MAX` (consumes no slot) but `tick_attribute`
*is* read. Slot trace:

| Wire slot | Value     | Field consumed                 |
|-----------|-----------|--------------------------------|
| 1         | `21`      | `message.skip()` (msg type)    |
| —         | —         | `message_version = i32::MAX` (no slot consumed) |
| 2         | `6`       | `message.skip()` (req id)      |
| 3         | `9000`    | `tick_type` (= `TickType::from(9000)`) |
| 4         | `13`      | `tick_attribute` (version ≥ 156) |
| 5         | `0.3`     | `implied_vol`                  |
| 6         | `0.35`    | `delta`                        |
| 7         | `5.25`    | `option_price`                 |
| 8         | `-0.025`  | `pv_dividend`                  |
| 9         | `0.55`    | `gamma`                        |
| 10        | `-0.0015` | `vega`                         |
| 11        | `0.3`     | `theta`                        |
| 12        | `4.75`    | `underlying_price`             |
| 13–14     | `140.0`, `5.25` | unread                   |

Assertions check `option_price == 5.25`, `delta == 0.35` — both match. Note
that `tick_type` here is `9000` (not `13` as in (a)/(b)) and the order of
fields after the tick_type slot shifts by one because slot 4 is now
`tick_attribute`, not the start of the optional-doubles run. Proto builder:

```rust
proto_response(
    IncomingMessages::TickOptionComputation,
    tick_option_computation()
        .request_id(9000)  // arbitrary; only echoed through, not asserted
        .tick_type(9000)
        // .tick_attrib(13) — only set if your builder exposes this; assertion
        //                   doesn't check it, so omitting is fine
        .implied_volatility(0.3)
        .delta(0.35)
        .option_price(5.25)
        .present_value_dividend(-0.025)
        .gamma(0.55)
        .vega(-0.0015)
        .theta(0.3)
        .underlying_price(4.75)
        .encode_proto(),
)
```

The `StreamDecoderTestCase.message: ResponseMessage` field shape stays the
same; only the content of the option-computation case changes from
`text_response(...)` to `proto_response(...)`. The other 3 cases in this fixture
list (option_chain, two error cases) stay as-is — option_chain is already
proto, and the error cases (`option chain end of stream` and `unexpected
message type`) drive `Err(Error::EndOfStream)` and `Err(Error::UnexpectedResponse)`
paths that are intentionally text-framed to test the *fallback* behavior;
keep them text-framed and verify they still skip-classify after the dispatch
change.

#### (d) Server-version hygiene

After step (c) the `server_version` value passed to `Client::stubbed` /
`DecoderContext::new` is decorative — the proto decoder doesn't read it.
Plan: leave the existing values (`REQ_CALC_OPTION_PRICE`, `SIZE_RULES`) since
changing them is a separate concern (request-encoding paths may still care).
Optional follow-up: bump these to `PROTOBUF_SCAN_DATA` (210) for consistency
with the rest of the proto-only test surface. Not blocking for this PR.

#### (e) Consumer updates

- `src/contracts/sync/tests.rs:82` — `MessageBusStub::with_responses(vec![test_case.response_message.clone()])`
  → `MessageBusStub::with_ordered_responses(test_case.ordered_responses.clone())`
- `src/contracts/async/tests.rs:87` (matching line) — same
- `src/contracts/sync/tests.rs:278` — `MessageBusStub::with_responses(test_case.response_messages.clone())`
  → `MessageBusStub::with_ordered_responses(test_case.ordered_responses.clone())`
- `src/contracts/sync/tests.rs:195` (`stream_decoder_test_cases` consumer) —
  the `message` field on the struct stays the same shape; only the content
  changes from `text_response(...)` to `proto_response(...)`. No consumer change.
- `src/contracts/async/tests.rs:209` — same

Update the comment at lines 495-496 and 727-729 of `test_tables.rs` (both
explicitly call out tick_option_computation as staying text-framed; both go).

### 4. Add the testdata response builder

Add to `src/testdata/builders/contracts.rs` (file already exists; check current
shape and follow it — the news/scanner builders are the precedent if not):

```rust
/// Builder for `TickOptionComputation` (msg 21) responses.
#[derive(Clone, Debug, Default)]
pub struct TickOptionComputationResponse {
    pub request_id: i32,
    pub tick_type: i32,
    pub tick_attrib: Option<i32>,
    pub implied_volatility: Option<f64>,
    pub delta: Option<f64>,
    pub option_price: Option<f64>,
    pub present_value_dividend: Option<f64>,
    pub gamma: Option<f64>,
    pub vega: Option<f64>,
    pub theta: Option<f64>,
    pub underlying_price: Option<f64>,
}

impl TickOptionComputationResponse {
    pub fn request_id(mut self, v: i32) -> Self { ... }
    pub fn tick_type(mut self, v: i32) -> Self { ... }
    pub fn implied_volatility(mut self, v: f64) -> Self { self.implied_volatility = Some(v); self }
    // ... one setter per Option<f64> field, all storing Some(v)
}

impl ResponseProtoEncoder for TickOptionComputationResponse {
    type Proto = proto::TickOptionComputation;

    fn to_proto(&self) -> Self::Proto {
        proto::TickOptionComputation {
            req_id: Some(self.request_id),
            tick_type: Some(self.tick_type),
            tick_attrib: self.tick_attrib,
            implied_vol: self.implied_volatility,
            delta: self.delta,
            opt_price: self.option_price,
            pv_dividend: self.present_value_dividend,
            gamma: self.gamma,
            vega: self.vega,
            theta: self.theta,
            und_price: self.underlying_price,
        }
    }
}

pub fn tick_option_computation() -> TickOptionComputationResponse {
    TickOptionComputationResponse::default()
}
```

**Builder home — recommend `market_data.rs`**. The news precedent
(`testdata/builders/news.rs` hosts the builders for the proto decoders in
`news/common/decoders.rs`) suggests "builder lives where the proto decoder
lives." Since `decode_tick_option_computation_proto` is in
`market_data/realtime/`, the builder belongs in `testdata/builders/market_data.rs`,
not `contracts.rs`. The only consumer is contracts tests today, which is fine —
contracts tests already import cross-domain (`market_data_request` is imported
into news tests at `news/sync_tests.rs:8`). Reviewer can flip this to
`contracts.rs` if they weigh "consumer locality" over "decoder locality" — flag
it in the PR description.

### 5. Doc comment cleanup

The `decoders/mod.rs:5-9` comment block (the general "all originating gates
≤ 210" claim) still applies after the migration — it doesn't single out
`decode_option_computation`. Re-read it during impl; likely no change needed.
What *does* need cleanup: the comment line in test_tables.rs (covered in
step 3) and the deletion of any `// Stays text-framed` markers around the
deleted function.

### 6. Sweep + tracker update

Same shell sweep as PR A. Tracker updates:

- Move `decode_option_computation` from "Decoders whose text branch is now
  unreachable at floor 210" → the "Floor 210 deletions (shipped)" list
- Update per-domain table — `contracts/common/decoders/` goes from
  "1 text-decoder, 4 proto-decoders, 0 dual-format" to "0 / 4 / 0" (the
  decoder count drops because the function is deleted outright, not
  collapsed to a proto wrapper — contracts no longer owns this message's
  decode path)
- Note that `next_optional_double` helper and the realtime↔contracts
  re-export are the structural change

---

## Out of scope (next ratchet candidate after both PRs)

Floor ratchet 210 → 211 (`PROTOBUF_REST_MESSAGES_1`) — the next floor bump
per `plans/legacy-text-protocol-cleanup.md`. With these two PRs landed, every
decoder at gate ≤ 210 is proto-only, so the floor is structurally clean to
ratchet.
