# Troubleshooting Guide

Common issues and solutions when using rust-ibapi.

## Build & Compilation Issues

### No Feature Specified

**Error:**
```
error: no feature specified. Enable either 'sync' or 'async' feature
```

**Solution:**
If you've disabled default features, add one back explicitly:
```bash
cargo build --features sync
# OR
cargo build --features async
```

### Mutually Exclusive Features

**Error:**
```
error: features 'sync' and 'async' are mutually exclusive
```

**Solution:**
Update to the latest `rust-ibapi`—current releases allow both features to be enabled together. If you still see this error, ensure `Cargo.lock` is refreshed:
```bash
cargo update -p ibapi
```

## Connection Issues

### Connection Refused

**Error:**
```
Error: Connection refused (os error 111)
```

**Solutions:**
1. **Start IB Gateway/TWS** - Ensure it's running
2. **Check port number:**
   - IB Gateway Paper: 4002
   - IB Gateway Live: 4001
   - TWS Paper: 7497
   - TWS Live: 7496
3. **Verify API is enabled** in IB Gateway/TWS settings

### API Not Configured

**Error:**
```
Error: API connection not configured
```

**Solution:**
Configure IB Gateway/TWS:
1. Go to **Configuration → API → Settings**
2. Enable **"Enable ActiveX and Socket Clients"**
3. Add **127.0.0.1** to trusted IPs
4. Disable **"Read-Only API"** if you need to place orders
5. Set **Master API client ID** if needed

### Connection Timeout

**Error:**
```
Error: Connection timeout
```

**Solutions:**
1. Check firewall settings
2. Verify IB Gateway/TWS is accepting connections
3. Ensure correct IP address (usually 127.0.0.1 for local)
4. Try increasing connection timeout in code

### Unrecognized Timezone From IB Gateway

**Error:**
```
unrecognized IB Gateway timezone "Some Standard Time"; register a mapping with
`ibapi::register_timezone_alias("Some Standard Time", "<IANA-name>")` ...
```

IB Gateway sends a free-form timezone string from the host machine's OS locale.
On non-English Windows installations the string may be a Windows TZ name we
don't recognize, mojibake from a non-UTF-8 locale, or any other label produced
by the gateway's environment. The crate ships with a built-in mapping table,
but you can extend it at runtime without rebuilding.

**Option 1 — programmatic (one mapping at a time):**
```rust
ibapi::register_timezone_alias("Some Standard Time", "America/New_York");
let client = Client::connect("127.0.0.1:4002", 100)?;
```

Call `register_timezone_alias` before `Client::connect`. User-registered
aliases override the built-ins, so you can correct a default you disagree
with as well as add new ones.

**Option 2 — environment variable (multiple mappings, no code change):**
```bash
export IBAPI_TIMEZONE_ALIASES="Some Standard Time=America/New_York;Other=Europe/Berlin"
cargo run --example connect
```

The format is `name=iana` pairs separated by `;`. Whitespace around tokens is
trimmed. Malformed entries are logged at warn level and skipped.

To verify a mapping is being applied, run with `RUST_LOG=ibapi::common::timezone=debug`
and look for `timezone alias matched (registry): ...`.

If you find a label that should be a built-in (a common Windows locale name
not yet in our table), please file an issue at
<https://github.com/wboayue/rust-ibapi/issues> with the exact string the
gateway is sending.

## Market Data Issues

### No Market Data Permissions

**Error:**
```
Error: No market data permissions for SYMBOL
```

**Solution:**
Your IB account needs market data subscriptions:
1. Log into IB Account Management
2. Subscribe to required market data
3. Wait for activation (can take a few minutes)
4. Restart IB Gateway/TWS

### Delayed Data Only

**Symptom:** Receiving 15-minute delayed data

**Solution:**
1. Subscribe to real-time data for the exchange
2. For paper trading, ensure you have paper trading market data enabled
3. Use `req_market_data_type(MarketDataType::Delayed)` to explicitly request delayed data

## Order Issues

### Invalid Order ID

**Error:**
```
Error: Invalid order ID
```

**Solution:**
Always use `client.next_order_id()` to get a valid order ID:
```rust
let order_id = client.next_order_id();
client.place_order(order_id, &contract, &order)?;
```

### Order Rejected

**Error:**
```
Error: Order rejected - reason: ...
```

**Common Reasons:**
1. **Insufficient funds** - Check account balance
2. **Invalid symbol** - Verify contract details
3. **Market closed** - Check trading hours
4. **Invalid order type** - Some order types require specific exchanges
5. **Position limits** - Check account restrictions

## Runtime Issues

### Subscription Ends Immediately

**Symptom:** Market data subscription returns no data and ends

**Solutions:**
1. Check if market is open
2. Verify contract is valid
3. Ensure market data permissions
4. Check for error messages in debug logs

### Missing Messages

**Symptom:** Not receiving expected responses

**Solution:**
Enable debug logging to see all messages:
```bash
RUST_LOG=debug cargo run --features sync --example your_example
```

### Slow Performance

**Solutions:**
1. Use async mode for high-concurrency scenarios
2. Reduce logging level in production
3. Use batch operations where available
4. Check network latency to IB servers

## Debugging Techniques

### Enable Debug Logging

See detailed communication with TWS:
```bash
# Debug level
RUST_LOG=debug cargo run --features sync --example your_example

# Trace level (very verbose)
RUST_LOG=trace cargo run --features sync --example your_example

# Only ibapi debug messages
RUST_LOG=ibapi=debug cargo run --features sync --example your_example
```

### Record TWS Messages

Save all TWS communication for analysis:
```bash
IBAPI_RECORDING_DIR=/tmp/tws-messages cargo run --features sync --example your_example

# View recorded messages
ls -la /tmp/tws-messages/
cat /tmp/tws-messages/requests.txt
cat /tmp/tws-messages/responses.txt
```

### Common Debug Patterns

```rust
// Add debug output to your code
dbg!(&contract);
println!("Order state: {:?}", order_status);

// Use expect for better error messages
let client = Client::connect("127.0.0.1:4002", 100)
    .expect("Failed to connect to IB Gateway");

// Handle specific errors
match client.place_order(order_id, &contract, &order) {
    Ok(_) => println!("Order placed successfully"),
    Err(Error::InvalidOrderId) => println!("Invalid order ID"),
    Err(e) => println!("Order failed: {}", e),
}
```

## Testing Issues

### Tests Failing in CI

**Common Causes:**
1. Not testing both feature modes
2. Network-dependent tests without mocking
3. Time-dependent tests

**Solution:**
```bash
# Always test both modes locally before pushing
cargo test --features sync
cargo test --features async
cargo clippy --features sync
cargo clippy --features async
```

### MockGateway Tests Failing

**Solution:**
Ensure test data matches expected format:
```rust
// Use exact message format from TWS
let response = "1|2|9000|AAPL|STK||0.0|||||NASDAQ|USD|AAPL|NMS|...|";
```

## Platform-Specific Issues

### macOS: Security Warnings

**Issue:** macOS blocks IB Gateway from accepting connections

**Solution:**
1. Go to System Preferences → Security & Privacy
2. Allow IB Gateway to accept incoming connections
3. Or run IB Gateway with reduced security (development only)

### Linux: Permission Denied

**Issue:** Cannot bind to port

**Solution:**
1. Run IB Gateway as your user (not root)
2. Ensure port is not already in use: `lsof -i :4002`
3. Check SELinux/AppArmor policies if enabled

### Windows: Firewall Blocking

**Issue:** Windows Firewall blocking connections

**Solution:**
1. Add IB Gateway to firewall exceptions
2. Allow localhost connections
3. Temporarily disable firewall for testing (not recommended for production)

## Getting Further Help

If you're still stuck:

1. **Search existing issues**: [GitHub Issues](https://github.com/wboayue/rust-ibapi/issues)
2. **Check examples**: Most common patterns are demonstrated
3. **Read test cases**: Tests show expected behavior
4. **Enable debug logging**: Often reveals the root cause
5. **Create an issue**: Include:
   - Rust version (`rustc --version`)
   - Feature flag used (sync or async)
   - Minimal code to reproduce
   - Full error message
   - Debug log output

## Quick Fixes Checklist

- [ ] Using exactly one feature flag (sync OR async)?
- [ ] IB Gateway/TWS running?
- [ ] Correct port number?
- [ ] API enabled in IB Gateway/TWS?
- [ ] Market data subscriptions active?
- [ ] Using `client.next_order_id()` for orders?
- [ ] Market open for the symbol?
- [ ] Debug logging enabled?
- [ ] Checked examples for similar use case?
- [ ] Tested both sync and async modes?
