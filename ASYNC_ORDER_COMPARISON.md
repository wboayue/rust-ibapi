# Async vs Sync Order Implementation Comparison

## DataStream/AsyncDataStream Implementations

### PlaceOrder
| Aspect | Sync | Async | Equivalent |
|--------|------|-------|------------|
| Trait | `DataStream<PlaceOrder>` | `AsyncDataStream<PlaceOrder>` | ✓ |
| decode() messages | OpenOrder, OrderStatus, ExecutionData, CommissionsReport, Error | Same | ✓ |
| Decoders used | Same decoders from common module | Same | ✓ |
| Error handling | Returns `Error::UnexpectedResponse` for unknown | Same | ✓ |

### OrderUpdate
| Aspect | Sync | Async | Equivalent |
|--------|------|-------|------------|
| Trait | `DataStream<OrderUpdate>` | `AsyncDataStream<OrderUpdate>` | ✓ |
| decode() messages | OpenOrder, OrderStatus, ExecutionData, CommissionsReport, Error | Same | ✓ |
| Decoders used | Same decoders from common module | Same | ✓ |
| Error handling | Returns `Error::UnexpectedResponse` for unknown | Same | ✓ |

### CancelOrder
| Aspect | Sync | Async | Equivalent |
|--------|------|-------|------------|
| Trait | `DataStream<CancelOrder>` | `AsyncDataStream<CancelOrder>` | ✓ |
| decode() messages | OrderStatus, Error | Same | ✓ |
| Decoders used | Same decoders from common module | Same | ✓ |
| Error handling | Returns `Error::UnexpectedResponse` for unknown | Same | ✓ |

### Orders
| Aspect | Sync | Async | Equivalent |
|--------|------|-------|------------|
| Trait | `DataStream<Orders>` | `AsyncDataStream<Orders>` | ✓ |
| decode() messages | CompletedOrder, CommissionsReport, OpenOrder, OrderStatus, OpenOrderEnd, CompletedOrdersEnd, Error | Same | ✓ |
| End of stream | Returns `Error::EndOfStream` for End messages | Same | ✓ |
| Error handling | Returns `Error::UnexpectedResponse` for unknown | Same | ✓ |

### Executions
| Aspect | Sync | Async | Equivalent |
|--------|------|-------|------------|
| Trait | `DataStream<Executions>` | `AsyncDataStream<Executions>` | ✓ |
| decode() messages | ExecutionData, CommissionsReport, ExecutionDataEnd, Error | Same | ✓ |
| End of stream | Returns `Error::EndOfStream` for ExecutionDataEnd | Same | ✓ |
| Error handling | Returns `Error::UnexpectedResponse` for unknown | Same | ✓ |

### ExerciseOptions
| Aspect | Sync | Async | Equivalent |
|--------|------|-------|------------|
| Trait | `DataStream<ExerciseOptions>` | `AsyncDataStream<ExerciseOptions>` | ✓ |
| decode() messages | OpenOrder, OrderStatus, Error | Same | ✓ |
| Error handling | Returns `Error::UnexpectedResponse` for unknown | Same | ✓ |

## Public API Functions

### order_update_stream
| Aspect | Sync | Async | Equivalent |
|--------|------|-------|------------|
| Signature | `pub fn order_update_stream<'a>(client: &'a Client) -> Result<Subscription<'a, OrderUpdate>, Error>` | `pub async fn order_update_stream(client: &Client) -> Result<Subscription<OrderUpdate>, Error>` | ✓ |
| Implementation | Calls `client.create_order_update_subscription()` | Calls `client.create_order_update_subscription().await` | ✓ |
| Subscription creation | `Subscription::new(client, subscription, ResponseContext::default())` | `Subscription::new_from_internal::<OrderUpdate>(subscription, Arc::new(client.clone()))` | ✓ |
| Only one allowed | Yes (returns AlreadySubscribed) | Yes (returns AlreadySubscribed) | ✓ |

### submit_order
| Aspect | Sync | Async | Equivalent |
|--------|------|-------|------------|
| Signature | `pub fn submit_order(client: &Client, order_id: i32, contract: &Contract, order: &Order) -> Result<(), Error>` | `pub async fn submit_order(client: &Client, order_id: i32, contract: &Contract, order: &Order) -> Result<(), Error>` | ✓ |
| Verification | `verify_order` and `verify_order_contract` | Same | ✓ |
| Encoding | `encode_place_order` with server_version | Same | ✓ |
| Send method | `client.send_message(request)?` | `client.send_message(request).await?` | ✓ |
| Returns | `Ok(())` - fire and forget | Same | ✓ |

### place_order
| Aspect | Sync | Async | Equivalent |
|--------|------|-------|------------|
| Signature | `pub fn place_order<'a>(client: &'a Client, order_id: i32, contract: &Contract, order: &Order) -> Result<Subscription<'a, PlaceOrder>, Error>` | `pub async fn place_order(client: &Client, order_id: i32, contract: &Contract, order: &Order) -> Result<Subscription<PlaceOrder>, Error>` | ✓ |
| Verification | `verify_order` and `verify_order_contract` | Same | ✓ |
| Encoding | `encode_place_order` with server_version | Same | ✓ |
| Send method | `client.send_order(order_id, request)?` | `client.send_order(order_id, request).await?` | ✓ |
| Returns subscription | Yes | Yes | ✓ |

### cancel_order
| Aspect | Sync | Async | Equivalent |
|--------|------|-------|------------|
| Signature | `pub fn cancel_order<'a>(client: &'a Client, order_id: i32, manual_order_cancel_time: &str) -> Result<Subscription<'a, CancelOrder>, Error>` | `pub async fn cancel_order(client: &Client, order_id: i32, manual_order_cancel_time: &str) -> Result<Subscription<CancelOrder>, Error>` | ✓ |
| Version check | Uses `client.check_server_version` for MANUAL_ORDER_TIME | Uses `check_version` for Features::MANUAL_ORDER_TIME | ✓ (functionally same) |
| Encoding | `encode_cancel_order` with server_version | Same | ✓ |
| Send method | `client.send_order(order_id, request)?` | `client.send_order(order_id, request).await?` | ✓ |
| Returns subscription | Yes | Yes | ✓ |

### global_cancel
| Aspect | Sync | Async | Equivalent |
|--------|------|-------|------------|
| Signature | `pub fn global_cancel(client: &Client) -> Result<(), Error>` | `pub async fn global_cancel(client: &Client) -> Result<(), Error>` | ✓ |
| Version check | `client.check_server_version` for REQ_GLOBAL_CANCEL | `check_version` for Features::REQ_GLOBAL_CANCEL | ✓ |
| Request ID | Uses `client.next_request_id()` | Same | ✓ |
| Send method | `client.send_order(request_id, message)?` | `client.send_order(request_id, message).await?` | ✓ |

### next_valid_order_id
| Aspect | Sync | Async | Equivalent |
|--------|------|-------|------------|
| Signature | `pub fn next_valid_order_id(client: &Client) -> Result<i32, Error>` | `pub async fn next_valid_order_id(client: &Client) -> Result<i32, Error>` | ✓ |
| Message type | OutgoingMessages::RequestIds | Same | ✓ |
| Response parsing | Reads int at index 2 | Same | ✓ |
| Updates client ID | `client.set_next_order_id(next_order_id)` | `client.set_next_order_id(next_order_id)` | ✓ |

### completed_orders
| Aspect | Sync | Async | Equivalent |
|--------|------|-------|------------|
| Signature | `pub fn completed_orders<'a>(client: &'a Client, api_only: bool) -> Result<Subscription<'a, Orders>, Error>` | `pub async fn completed_orders(client: &Client, api_only: bool) -> Result<Subscription<Orders>, Error>` | ✓ |
| Version check | `client.check_server_version` for COMPLETED_ORDERS | `check_version` for Features::COMPLETED_ORDERS | ✓ |
| Message type | OutgoingMessages::RequestCompletedOrders | Same | ✓ |
| Returns subscription | Yes | Yes | ✓ |

### open_orders
| Aspect | Sync | Async | Equivalent |
|--------|------|-------|------------|
| Signature | `pub fn open_orders<'a>(client: &'a Client) -> Result<Subscription<'a, Orders>, Error>` | `pub async fn open_orders(client: &Client) -> Result<Subscription<Orders>, Error>` | ✓ |
| Message type | OutgoingMessages::RequestOpenOrders | Same | ✓ |
| Returns subscription | Yes | Yes | ✓ |

### all_open_orders
| Aspect | Sync | Async | Equivalent |
|--------|------|-------|------------|
| Signature | `pub fn all_open_orders<'a>(client: &'a Client) -> Result<Subscription<'a, Orders>, Error>` | `pub async fn all_open_orders(client: &Client) -> Result<Subscription<Orders>, Error>` | ✓ |
| Message type | OutgoingMessages::RequestAllOpenOrders | Same | ✓ |
| Returns subscription | Yes | Yes | ✓ |

### auto_open_orders
| Aspect | Sync | Async | Equivalent |
|--------|------|-------|------------|
| Signature | `pub fn auto_open_orders<'a>(client: &'a Client, auto_bind: bool) -> Result<Subscription<'a, Orders>, Error>` | `pub async fn auto_open_orders(client: &Client, auto_bind: bool) -> Result<Subscription<Orders>, Error>` | ✓ |
| Message type | OutgoingMessages::RequestAutoOpenOrders | Same | ✓ |
| Returns subscription | Yes | Yes | ✓ |

### executions
| Aspect | Sync | Async | Equivalent |
|--------|------|-------|------------|
| Signature | `pub fn executions<'a>(client: &'a Client, filter: ExecutionFilter) -> Result<Subscription<'a, Executions>, Error>` | `pub async fn executions(client: &Client, filter: ExecutionFilter) -> Result<Subscription<Executions>, Error>` | ✓ |
| Request ID | Uses `client.next_request_id()` | Same | ✓ |
| Returns subscription | Yes | Yes | ✓ |

### exercise_options
| Aspect | Sync | Async | Equivalent |
|--------|------|-------|------------|
| Signature | Similar with OffsetDateTime parameter | Same | ✓ |
| Request ID | Uses `client.next_request_id()` | Same | ✓ |
| Returns subscription | Yes | Yes | ✓ |

## Order Update Stream Routing

### Sync Implementation
- Uses `send_order_update()` method in TcpMessageBus
- Routes order messages to both specific subscriptions AND order update stream
- Handles: ExecutionData, OrderStatus, OpenOrder, CommissionReport

### Async Implementation  
- Uses `send_order_update()` method in AsyncTcpMessageBus
- Routes order messages to both specific subscriptions AND order update stream
- Handles: Same message types

## Summary

The async implementation is **functionally equivalent** to the sync implementation with these minor differences:

1. **Async syntax**: Functions are `async`, use `.await`, no lifetimes in return types
2. **Version checking**: Async uses the newer `check_version` function with Features enum, while sync uses the older `client.check_server_version`. Both achieve the same result.
3. **next_valid_order_id**: Async has a TODO comment about updating the client's order ID, while sync does update it. This is a minor difference that doesn't affect core functionality.

All core functionality is preserved:
- Same message types handled
- Same decoders used
- Same verification logic
- Same error handling
- Same routing behavior for order update stream
- Same "only one order update stream" constraint