//! A prelude module for convenient importing of commonly used types and traits.
//!
//! This module re-exports the most frequently used types from the ibapi crate
//! to simplify imports in user code. Instead of importing each type individually:
//!
//! ```rust
//! # #![allow(unused_imports)]
//! use ibapi::client::Client;
//! use ibapi::contracts::Contract;
//! use ibapi::orders::{Action, PlaceOrder};
//! use ibapi::market_data::historical::{BarSize, WhatToShow, ToDuration};
//! ```
//!
//! You can simply use:
//!
//! ```rust
//! # #![allow(unused_imports)]
//! use ibapi::prelude::*;
//! ```
//!
//! ## Type naming: `BarSize` and `WhatToShow`
//!
//! Both `market_data::historical` and `market_data::realtime` define their own
//! `BarSize` and `WhatToShow` enums (different variant sets — historical has 21
//! `BarSize` variants and 10 `WhatToShow` variants; realtime has only `Sec5` and
//! a 4-variant subset). Two canonical spellings depending on import style:
//!
//! - **Prelude (flat) imports** — use the disambiguated names
//!   `HistoricalBarSize` / `HistoricalWhatToShow` / `RealtimeBarSize` /
//!   `RealtimeWhatToShow`. These are the canonical names for
//!   `use ibapi::prelude::*;` callers.
//! - **Module-qualified imports** — use the short names directly:
//!   `use ibapi::market_data::historical::{BarSize, WhatToShow};` or
//!   `use ibapi::market_data::realtime::{BarSize, WhatToShow};`. The module
//!   path provides the namespace; the short name is idiomatic Rust.
//!
//! Both spellings refer to the same type — the prelude entries are `pub use`
//! re-exports with `as` aliasing, not separate types.

// Core client
pub use crate::Client;
pub use crate::ClientBuilder;
pub use crate::Error;
pub use crate::{Notice, NoticeCategory};

// Contract types
pub use crate::contracts::{BondIdentifier, ContractMonth, Currency, Cusip, Exchange, ExpirationDate, Isin, LegAction, OptionRight, Strike, Symbol};
pub use crate::contracts::{Contract, SecurityType};

// Market data types - historical
pub use crate::market_data::historical::{
    BarSize as HistoricalBarSize, HistoricalDataBuilder, HistoricalScheduleBuilder, HistoricalTicksBuilder, IgnoreSize, ToDuration,
    WhatToShow as HistoricalWhatToShow,
};

// Market data types - realtime
pub use crate::market_data::realtime::{BarSize as RealtimeBarSize, TickTypes, WhatToShow as RealtimeWhatToShow};
pub use crate::market_data::{MarketDataType, TradingHours};

// Order types
pub use crate::orders::{order_builder, Action, ExecutionFilter, ExecutionFilterSide, ExecutionSide, OrderUpdate, Orders, PlaceOrder};

// Account types
pub use crate::accounts::{
    AccountSummaryResult, AccountSummaryTags, AccountUpdate, AccountUpdateMulti, FamilyCode, PnL, PnLSingle, PositionUpdate, PositionUpdateMulti,
};

// Subscription types (canonical home: crate::subscriptions)
#[cfg(feature = "async")]
pub use crate::subscriptions::SubscriptionItemStreamExt;
pub use crate::subscriptions::{NoticeStream, Subscription, SubscriptionItem};

// Async-specific imports
#[cfg(feature = "async")]
pub use futures::StreamExt;
