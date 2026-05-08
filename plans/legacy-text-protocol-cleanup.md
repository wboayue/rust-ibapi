# Legacy Text-Protocol Cleanup Tracker

**End goal:** support protobuf-only on the wire (both directions) and delete every text-protocol code path from the crate.

## Status today

- **Outgoing:** all requests are protobuf. The text encoders are gone (PRs #449–#452, summarized in [protobuf-migration.md](protobuf-migration.md)).
- **Incoming:** decoders are still mostly text-only with a small number of dual-format (`decode_proto_or_text`) call sites. The wire flips a message family from text to protobuf at a per-family server-version gate, so text-decode code stays load-bearing until the minimum server version we accept covers every gate.
- **Connection gate:** `connection::common::require_protobuf_support` rejects servers below `server_versions::PROTOBUF_PLACE_ORDER` (203) — gate added in [#492](https://github.com/wboayue/rust-ibapi/pull/492); floor ratcheted from 201 → 203 in this PR. Decoders weren't deleted as part of the bump — that's a follow-up PR after each family's response-format mapping is grounded in captured wire data (the `OutgoingMessages`-based grouping in C# Constants.cs maps to outgoing requests, not the responses we decode). Next ratchet candidate: 204 (`PROTOBUF_COMPLETED_ORDER`).

## Per-family protobuf-incoming gates

From `src/server_versions.rs`. A response in family X is protobuf iff `server_version >= GATE[X]`. We can delete that family's text branch only after the floor passes the gate.

| Family                  | Constant                       | Min version |
|-------------------------|--------------------------------|------------:|
| Connection (StartApi)   | `PROTOBUF`                     |         201 |
| PlaceOrder              | `PROTOBUF_PLACE_ORDER`         |         203 |
| CompletedOrder          | `PROTOBUF_COMPLETED_ORDER`     |         204 |
| Contract data           | `PROTOBUF_CONTRACT_DATA`       |         205 |
| Market data             | `PROTOBUF_MARKET_DATA`         |         206 |
| Accounts & positions    | `PROTOBUF_ACCOUNTS_POSITIONS`  |         207 |
| Historical data         | `PROTOBUF_HISTORICAL_DATA`     |         208 |
| News                    | `PROTOBUF_NEWS_DATA`           |         209 |
| Scanner                 | `PROTOBUF_SCAN_DATA`           |         210 |
| REST batch 1            | `PROTOBUF_REST_MESSAGES_1`     |         211 |
| REST batch 2            | `PROTOBUF_REST_MESSAGES_2`     |         212 |
| REST batch 3            | `PROTOBUF_REST_MESSAGES_3`     |         213 |

End-state: raise the floor to `PROTOBUF_REST_MESSAGES_3` (213). Every text-decode branch becomes dead code and can be deleted along with the helpers below.

## Inventory of legacy text-protocol surface

### Decoders that still parse text

Per-domain counts. "Text-decoders" = functions consuming `ResponseMessage`
field-by-field (`message.skip()`, `next_string()`, …). "Proto-decoders" =
functions consuming protobuf bytes. "Dual-format calls" = number of
`decode_proto_or_text{,_owned}` call sites in the module — the pre-existing
text decoders are still load-bearing for servers below the family's gate.

| Domain                                    | Text-decoders | Proto-decoders | Dual-format calls |
|-------------------------------------------|--------------:|---------------:|------------------:|
| `accounts/common/decoders/`               |            14 |             10 |                12 |
| `contracts/common/decoders/`              |             5 |              4 |                 4 |
| `orders/common/decoders/`                 |            16 |              5 |                 7 |
| `market_data/realtime/common/decoders/`   |            15 |             10 |                 1 |
| `market_data/historical/common/decoders/` |             8 |             10 |                 9 |
| `news/common/decoders.rs`                 |             5 |              4 |                 4 |
| `scanner/common/decoders.rs`              |             3 |              2 |                 2 |
| `wsh/common/decoders.rs`                  |             3 |              2 |                 0 |
| `display_groups/common/decoders.rs`       |             1 |              1 |                 0 |

Most domains now have proto counterparts and `decode_proto_or_text` wrappers
in place — the remaining work in §"Per-domain done checklist" is mostly
*deleting* the text branch once the floor passes the family's gate, not
adding proto decoders. Realtime market data and orders still have the largest
text-decoder surface that hasn't been converted to dual-format wrappers yet.

### Helper APIs that go away when all decoders are proto-only

These exist solely to support text-format messages. Each can be deleted once no decoder reads from a text-format `ResponseMessage`.

- `messages::ResponseMessage::is_protobuf` field (`src/messages.rs:849`)
- `messages::ResponseMessage::from(fields: &str)` inherent constructor (`src/messages.rs:1231`) — note: not a `From<&str>` impl, despite the name
- `messages::ResponseMessage::from_binary_text` (`src/messages.rs:868`)
- `messages::ResponseMessage::with_server_version` (`src/messages.rs:1253`)
- `messages::ResponseMessage::decode_proto_or_text{,_owned}` (`src/messages.rs:886, 901`)
- `connection::common::parse_raw_message` text-payload branch (`src/connection/common.rs:451`)
- All `message.skip()` calls (currently used to skip the text-format `message_type` and `message_version` header fields)

### Branching sites in production code

`if message.is_protobuf` decisions outside the decoder bodies. Each disappears with the field.

- `src/messages.rs:891, 906` — inside `decode_proto_or_text{,_owned}`
- `src/messages.rs:1367` — inside `From<&ResponseMessage> for Notice`
- `src/errors.rs:116` — inside `From<ResponseMessage> for Error`
- `src/transport/routing.rs:120, 129` — error/notice routing
- `src/connection/common.rs:267, 280` — handshake `NextValidId` / `ManagedAccounts` parsing

### Sentinel-message uses of the text constructor

Production sentinels have moved off `ResponseMessage::from(&str)` — the
`Cancelled` / `ConnectionReset` paths now send `Error::*` directly via the
`From<Error> for RoutedItem` impl in `subscriptions/common.rs:67`. Remaining
production callers of `ResponseMessage::from(&str)` are limited to:

- `src/display_groups/common/decoders.rs:41` and `src/display_groups/common/stream_decoders.rs:51` — wrapping a parsed text payload after server-side framing; replace when display groups gets a proto decoder.
- `src/connection/common.rs:454` — text-path branch of `parse_raw_message`; goes away with the helper itself.
- `src/stubs.rs:76` — test-fixture-only.

The `"stray\0"` sentinel for `UnexpectedResponse` is now test-only
(`src/subscriptions/sync_tests.rs`, `src/subscriptions/async_tests.rs`); no
production code emits it.

## Strategy

Two viable paths, not mutually exclusive:

1. **Per-family ratchet.** Pick a family, bump the floor to its gate (e.g. raise `require_protobuf_support` minimum from 201 to 207 for accounts — extending the gate landed in [#492](https://github.com/wboayue/rust-ibapi/pull/492)), convert that domain's decoders to proto-only, delete the text branches and any `decode_proto_or_text` wrappers in that domain, ship. Repeat for the next family.
2. **Big-bang.** Raise the floor to 213 (`PROTOBUF_REST_MESSAGES_3`) in one PR, convert all remaining decoders to proto-only, delete the helpers, ship. Larger blast radius but ends the carrying cost in one move.

Either path ends at the same place: only the proto branches remain, the helpers in §"Helper APIs that go away" are deleted, and `ResponseMessage` collapses to a thin protobuf-payload carrier (or is replaced entirely).

## Per-domain "done" checklist

For each row in the decoder table:

1. Add a proto-decoder for every response type in the domain that doesn't already have one (mirror the patterns in `src/proto/decoders.rs` and the accounts dual-format trio). Most domains now have proto counterparts — see the proto-decoders column.
2. Wrap each domain decoder in `decode_proto_or_text` *or* delete the text branch outright (depending on whether the floor has passed the family's gate).
3. Raise the floor so the text branch is unreachable: bump the constant in `connection::common::require_protobuf_support` (the gate added by [#492](https://github.com/wboayue/rust-ibapi/pull/492)) to the family's `PROTOBUF_<FAMILY>` value, or — if the bump would be too aggressive globally — add a per-feature `check_version` call at the public API entry point.
4. Delete the text branches and update the corresponding `_tests.rs` to drive proto fixtures only.
5. `cargo test` (default + `--features sync` + `--all-features`), `cargo clippy --all-targets [-- -D warnings]` for each configuration, `cargo fmt`.
6. Update this file: drop the row from the inventory.

## Final-cleanup checklist (after all rows are gone)

- Delete the helpers listed under "Helper APIs that go away".
- Delete the `is_protobuf` branches listed under "Branching sites in production code".
- Replace the remaining `ResponseMessage::from(&str)` callers (display_groups decoders, `parse_raw_message` text branch) with proto equivalents, then delete the inherent `from(fields: &str)` constructor and `from_binary_text`.
- Simplify `ResponseMessage` to a protobuf-only payload carrier, or delete it in favor of using `prost`-decoded message types directly on the channels.
- Bump the major version if any of the above breaks public API (most of the helpers above are `pub`).

## Source of truth

The same C# constants file the outgoing tracker uses:
`/Users/wboayue/projects/tws-api/source/csharpclient/client/Constants.cs` (`PROTOBUF_MSG_IDS`).
A message family is "protobuf-incoming" iff its message ID appears in that map at the relevant min-server-version.
