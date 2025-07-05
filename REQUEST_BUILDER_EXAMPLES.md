# Request/Response Builder Pattern Examples

This document demonstrates how the request builder pattern simplifies client method implementations.

## Overview

The builder pattern reduces boilerplate code and provides a fluent API for:
- Version checking
- Request ID management  
- Message sending
- Subscription creation

## Example Refactorings

### 1. Shared Request (No Request ID)

**Original:**
```rust
pub(crate) fn positions(client: &Client) -> Result<Subscription<PositionUpdate>, Error> {
    client.check_server_version(server_versions::ACCOUNT_SUMMARY, "It does not support position requests.")?;

    let request = encoders::encode_request_positions()?;
    let subscription = client.send_shared_request(OutgoingMessages::RequestPositions, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}
```

**Refactored:**
```rust
pub(crate) fn positions(client: &Client) -> Result<Subscription<PositionUpdate>, Error> {
    let request = encoders::encode_request_positions()?;
    
    client
        .shared_request(OutgoingMessages::RequestPositions)
        .check_version(server_versions::ACCOUNT_SUMMARY, "It does not support position requests.")?
        .send(request)
}
```

### 2. Request with ID

**Original:**
```rust
pub(crate) fn pnl(client: &Client, account: &str, model_code: Option<&str>) -> Result<Subscription<PnL>, Error> {
    client.check_server_version(server_versions::PNL, "It does not support PnL requests.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_pnl(request_id, account, model_code)?;
    let subscription = client.send_request(request_id, request)?;

    Ok(Subscription::new(client, subscription, ResponseContext::default()))
}
```

**Refactored:**
```rust
pub(crate) fn pnl(client: &Client, account: &str, model_code: Option<&str>) -> Result<Subscription<PnL>, Error> {
    let builder = client
        .request()
        .check_version(server_versions::PNL, "It does not support PnL requests.")?;
    
    let request = encoders::encode_request_pnl(builder.request_id(), account, model_code)?;
    builder.send(request)
}
```

### 3. Request with Custom Context

**Original:**
```rust
pub(crate) fn contract_details(
    client: &Client,
    contract: &Contract,
) -> Result<Subscription<ContractDetails>, Error> {
    client.check_server_version(server_versions::BOND_ISSUER_ID, "It does not support contract details requests.")?;

    let request_id = client.next_request_id();
    let request = encoders::encode_request_contract_details(client.server_version, request_id, contract)?;
    let subscription = client.send_request(request_id, request)?;
    
    let context = ResponseContext {
        contract: Some(contract.clone()),
    };

    Ok(Subscription::new(client, subscription, context))
}
```

**Refactored:**
```rust
pub(crate) fn contract_details(
    client: &Client,
    contract: &Contract,
) -> Result<Subscription<ContractDetails>, Error> {
    let builder = client
        .request()
        .check_version(server_versions::BOND_ISSUER_ID, "It does not support contract details requests.")?;
    
    let request = encoders::encode_request_contract_details(client.server_version(), builder.request_id(), contract)?;
    let context = ResponseContext {
        contract: Some(contract.clone()),
    };
    
    builder.send_with_context(request, context)
}
```

### 4. Order Request

**Original:**
```rust
pub(crate) fn place_order(
    client: &Client,
    order_id: i32,
    contract: &Contract,
    order: &Order,
) -> Result<InternalSubscription, Error> {
    client.check_server_version(server_versions::ORDER_CONTAINER, "It does not support order placement.")?;
    
    let request = encoders::encode_place_order(client.server_version, order_id, contract, order)?;
    client.send_order(order_id, request)
}
```

**Refactored:**
```rust
pub(crate) fn place_order(
    client: &Client,
    order_id: i32,
    contract: &Contract,
    order: &Order,
) -> Result<InternalSubscription, Error> {
    let request = encoders::encode_place_order(client.server_version(), order_id, contract, order)?;
    
    client
        .order_request_with_id(order_id)
        .check_version(server_versions::ORDER_CONTAINER, "It does not support order placement.")?
        .send(request)
}
```

### 5. Simple Message (No Response)

**Original:**
```rust
pub(crate) fn request_global_cancel(client: &Client) -> Result<(), Error> {
    client.check_server_version(server_versions::REQ_GLOBAL_CANCEL, "It does not support global cancel.")?;
    
    let request = encoders::encode_request_global_cancel()?;
    client.send_message(request)
}
```

**Refactored:**
```rust
pub(crate) fn request_global_cancel(client: &Client) -> Result<(), Error> {
    let request = encoders::encode_request_global_cancel()?;
    
    client
        .message()
        .check_version(server_versions::REQ_GLOBAL_CANCEL, "It does not support global cancel.")?
        .send(request)
}
```

## Benefits

1. **Reduced Boilerplate**: Less repetitive code for common patterns
2. **Fluent API**: Chainable methods for better readability
3. **Consistent Error Handling**: Version checks integrated into the builder
4. **Flexibility**: Different builders for different use cases (request, shared request, order, message)
5. **Type Safety**: Compile-time guarantees for correct usage

## Usage

To use the builder pattern in your client methods:

1. Import the trait: `use crate::client::ClientRequestBuilders;`
2. Choose the appropriate builder method:
   - `client.request()` - For requests with auto-generated IDs
   - `client.shared_request(message_type)` - For requests without IDs
   - `client.order_request()` - For order requests
   - `client.message()` - For simple messages without responses
3. Chain operations as needed
4. Call `send()` or `send_with_context()` to execute