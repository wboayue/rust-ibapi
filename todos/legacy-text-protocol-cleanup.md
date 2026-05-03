# Legacy Text-Protocol Cleanup Tracker

**End goal:** support protobuf-only on the wire (both directions) and delete every text-protocol code path from the crate.

## Status today

- **Outgoing:** all requests are protobuf. The text encoders are gone (PRs #449–#452, summarized in [protobuf-migration.md](protobuf-migration.md)).
- **Incoming:** decoders are still mostly text-only with a small number of dual-format (`decode_proto_or_text`) call sites. The wire flips a message family from text to protobuf at a per-family server-version gate, so text-decode code stays load-bearing until the minimum server version we accept covers every gate.
- **Connection gate:** `connection::common::require_protobuf_support` rejects servers below `server_versions::PROTOBUF` (201) — added in [#492](https://github.com/wboayue/rust-ibapi/pull/492). That establishes the floor we can raise as families come over.

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

Per-domain counts of decoder functions that consume `ResponseMessage` field-by-field (`message.skip()`, `next_string()`, etc.). Each one needs a proto-decoder counterpart and either a `decode_proto_or_text` wrapper (transitional) or full replacement (after the gate).

| Domain                           | Text-only decoders | Dual-format (`decode_proto_or_text`) |
|----------------------------------|-------------------:|-------------------------------------:|
| `accounts/common/decoders/`      |                 17 |                                    3 |
| `contracts/common/decoders/`     |                  6 |                                    0 |
| `orders/common/decoders/`        |                 16 |                                    0 |
| `market_data/realtime/common/decoders/` |          21 |                                    0 |
| `market_data/historical/common/decoders/` |        15 |                                    0 |
| `news/common/decoders.rs`        |                  8 |                                    0 |
| `scanner/common/decoders.rs`     |                  4 |                                    0 |
| `display_groups/common/decoders.rs` |               0 |                                    0 |
| `wsh/common/decoders.rs`         |                  5 |                                    0 |

The three accounts dual-format decoders are the template for the rest:

- `accounts::common::decoders::decode_account_summary`
- `accounts::common::decoders::decode_server_time`
- `accounts::common::decoders::decode_server_time_millis`

### Helper APIs that go away when all decoders are proto-only

These exist solely to support text-format messages. Each can be deleted once no decoder reads from a text-format `ResponseMessage`.

- `messages::ResponseMessage::is_protobuf` field
- `messages::ResponseMessage::from(&str)` text constructor (and the `From<&str>` impl)
- `messages::ResponseMessage::from_binary_text` (`src/messages.rs:868`)
- `messages::ResponseMessage::with_server_version` (`src/messages.rs:1222`)
- `messages::ResponseMessage::decode_proto_or_text` (`src/messages.rs:886`)
- `connection::common::parse_raw_message` text-payload branch (`src/connection/common.rs:327`)
- All `message.skip()` calls (currently used to skip the text-format `message_type` and `message_version` header fields)

### Branching sites in production code

`if message.is_protobuf` decisions outside the decoder bodies. Each disappears with the field.

- `src/messages.rs:891` — inside `decode_proto_or_text`
- `src/transport/routing.rs:68, 84` — error/notice routing
- `src/connection/common.rs:184, 197, 210` — `NextValidId` / `ManagedAccounts` / `Error` parsing during handshake

### Sentinel-message uses of the text constructor

`ResponseMessage::from(&str)` is also used to fabricate in-process sentinels that never came from the wire. These need a different replacement (an enum variant on the channel, or a typed sentinel) before the text constructor can be deleted.

- `src/transport/async.rs:356, 364` — `"ConnectionReset"`
- `src/transport/async.rs:697, 709` — `"Cancelled"`
- `src/subscriptions/sync.rs:519`, `src/subscriptions/async.rs:475` — `"stray\0"` for `UnexpectedResponse`
- `src/transport/routing.rs:191` — wraps a stringified error before re-routing

## Strategy

Two viable paths, not mutually exclusive:

1. **Per-family ratchet.** Pick a family, bump the floor to its gate (e.g. raise `require_protobuf_support` minimum from 201 to 207 for accounts — extending the gate landed in [#492](https://github.com/wboayue/rust-ibapi/pull/492)), convert that domain's decoders to proto-only, delete the text branches and any `decode_proto_or_text` wrappers in that domain, ship. Repeat for the next family.
2. **Big-bang.** Raise the floor to 213 (`PROTOBUF_REST_MESSAGES_3`) in one PR, convert all remaining decoders to proto-only, delete the helpers, ship. Larger blast radius but ends the carrying cost in one move.

Either path ends at the same place: only the proto branches remain, the helpers in §"Helper APIs that go away" are deleted, and `ResponseMessage` collapses to a thin protobuf-payload carrier (or is replaced entirely).

## Per-domain "done" checklist

For each row in the decoder table:

1. Add a proto-decoder for every response type in the domain that doesn't already have one (mirror the patterns in `src/proto/decoders.rs` and the accounts dual-format trio).
2. Wrap each domain decoder in `decode_proto_or_text` *or* delete the text branch outright (depending on whether the floor has passed the family's gate).
3. Raise the floor so the text branch is unreachable: bump the constant in `connection::common::require_protobuf_support` (the gate added by [#492](https://github.com/wboayue/rust-ibapi/pull/492)) to the family's `PROTOBUF_<FAMILY>` value, or — if the bump would be too aggressive globally — add a per-feature `check_version` call at the public API entry point.
4. Delete the text branches and update the corresponding `_tests.rs` to drive proto fixtures only.
5. `cargo test` (default + `--features sync` + `--all-features`), `cargo clippy --all-targets [-- -D warnings]` for each configuration, `cargo fmt`.
6. Update this file: drop the row from the inventory.

## Final-cleanup checklist (after all rows are gone)

- Delete the helpers listed under "Helper APIs that go away".
- Delete the `is_protobuf` branches listed under "Branching sites in production code".
- Replace the sentinel uses of `ResponseMessage::from(&str)` with a typed channel-event enum or equivalent, then delete the `From<&str>` impl and the `from_binary_text` constructor.
- Simplify `ResponseMessage` to a protobuf-only payload carrier, or delete it in favor of using `prost`-decoded message types directly on the channels.
- Bump the major version if any of the above breaks public API (most of the helpers above are `pub`).

## Source of truth

The same C# constants file the outgoing tracker uses:
`/Users/wboayue/projects/tws-api/source/csharpclient/client/Constants.cs` (`PROTOBUF_MSG_IDS`).
A message family is "protobuf-incoming" iff its message ID appears in that map at the relevant min-server-version.
