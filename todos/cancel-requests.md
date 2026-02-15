# Complete cancel_contract_details and cancel_historical_ticks support

## Status
Implemented but requires server version >= 215. Current gateway supports up to 197.

## What's done
- Encoders, async/sync module functions, client methods, re-exports
- Integration tests (ignored until server supports v215)

## What's needed
- Bump max server version to 215+ once gateway supports it
- Un-ignore integration tests and verify against live gateway
- Consider adding cancel methods directly on `TickSubscription` so users don't need to track request IDs manually
