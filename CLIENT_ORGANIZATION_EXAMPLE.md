# Client Method Organization Approaches

## Current Situation
The Client struct has 63 public methods all in one file, making it large and harder to navigate.

## Approach 1: Moving Methods to Separate Files

This approach moves methods into separate module files based on functionality.

### Structure:
```
src/client/
├── sync.rs          (core Client struct and essential methods)
├── sync/
│   ├── accounts.rs  (account-related methods)
│   ├── orders.rs    (order-related methods)
│   └── market_data.rs (market data methods)
```

### Example - accounts.rs:
```rust
// src/client/sync/accounts.rs
use super::Client;
use crate::accounts::{AccountSummaries, PositionUpdate};
use crate::errors::Error;
use crate::subscriptions::Subscription;

impl Client {
    pub fn positions(&self) -> Result<Subscription<PositionUpdate>, Error> {
        // implementation
    }
    
    pub fn account_summary(&self, group: &str, tags: &str) -> Result<Subscription<AccountSummaries>, Error> {
        // implementation
    }
}
```

### Main file - sync.rs:
```rust
// src/client/sync.rs
mod accounts;  // brings in the impl block
mod orders;
mod market_data;

pub struct Client {
    // fields
}

impl Client {
    // Only core methods remain here
    pub fn connect(address: &str, client_id: i32) -> Result<Client, Error> { }
    pub fn server_version(&self) -> i32 { }
    pub fn next_order_id(&self) -> i32 { }
}
```

### Pros:
- Clean file organization
- Easy to find related methods
- Smaller, more focused files

### Cons:
- **BREAKS API**: Methods are no longer visible in documentation under Client
- Users can't see all available methods in one place
- IDE autocomplete might not work as expected
- Requires users to import traits (if we tried to fix with traits)

## Approach 2: Extension Traits with Implementations

This approach uses extension traits to organize methods while keeping the API intact.

### Structure:
```
src/client/
├── sync.rs          (Client struct + all public methods)
├── ext/
│   ├── accounts.rs  (AccountExt trait + implementations)
│   ├── orders.rs    (OrderExt trait + implementations)
│   └── market_data.rs (MarketDataExt trait + implementations)
```

### Example - accounts extension trait:
```rust
// src/client/ext/accounts.rs
use crate::Client;
use crate::accounts::{AccountSummaries, PositionUpdate};
use crate::errors::Error;
use crate::subscriptions::Subscription;
use crate::messages::{OutgoingMessages, RequestMessage};

pub trait AccountExt {
    fn positions(&self) -> Result<Subscription<PositionUpdate>, Error>;
    fn account_summary(&self, group: &str, tags: &str) -> Result<Subscription<AccountSummaries>, Error>;
}

impl AccountExt for Client {
    fn positions(&self) -> Result<Subscription<PositionUpdate>, Error> {
        let request_id = self.next_request_id();
        let message = RequestMessage::new(
            OutgoingMessages::ReqPositions,
            request_id,
            VERSION,
        );
        self.send_request(request_id, message)
    }
    
    fn account_summary(&self, group: &str, tags: &str) -> Result<Subscription<AccountSummaries>, Error> {
        let request_id = self.next_request_id();
        let mut message = RequestMessage::new(
            OutgoingMessages::ReqAccountSummary,
            request_id,
            VERSION,
        );
        message.push_field(&request_id);
        message.push_field(group);
        message.push_field(tags);
        self.send_request(request_id, message)
    }
}
```

### Main file - sync.rs:
```rust
// src/client/sync.rs
mod ext;
use ext::accounts::AccountExt;
use ext::orders::OrderExt;
use ext::market_data::MarketDataExt;

pub struct Client {
    // fields
}

impl Client {
    // Core methods
    pub fn connect(address: &str, client_id: i32) -> Result<Client, Error> { }
    pub fn server_version(&self) -> i32 { }
    pub fn next_order_id(&self) -> i32 { }
    
    // Delegate to extension traits
    pub fn positions(&self) -> Result<Subscription<PositionUpdate>, Error> {
        <Self as AccountExt>::positions(self)
    }
    
    pub fn account_summary(&self, group: &str, tags: &str) -> Result<Subscription<AccountSummaries>, Error> {
        <Self as AccountExt>::account_summary(self, group, tags)
    }
}
```

### Pros:
- **Preserves API**: All methods still appear on Client in docs
- Code is organized in separate files for maintainability
- No breaking changes for users
- Can share implementations between sync/async using the same trait

### Cons:
- More boilerplate (trait definition + impl + delegation)
- Slightly more complex structure
- Need to maintain both trait method and public method

## Recommendation

**Use Approach 2 (Extension Traits)** because:

1. It preserves backward compatibility
2. Keeps all methods discoverable on the Client struct
3. Allows internal organization without affecting users
4. Provides a path for sharing code between sync and async implementations
5. The extra boilerplate is a small price for maintaining a stable API

The extension trait approach gives us the internal organization benefits while keeping the external API unchanged. This is especially important for a library where API stability matters.