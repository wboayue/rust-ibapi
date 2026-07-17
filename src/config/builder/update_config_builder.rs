//! Fluent builder for `update_config` write requests.

use crate::config::common::encoders;
use crate::config::{ApiConfig, ConfigWarning, LockAndExit, MessageSetting, OrdersConfig};
use crate::proto;

/// Fluent builder for editing TWS/Gateway configuration.
///
/// Set only the groups you want to change; unset groups are left untouched by
/// the gateway. Terminate with [`submit`](Self::submit). Obtain one from
/// [`Client::update_config`](crate::Client::update_config).
#[must_use = "UpdateConfigBuilder does nothing until you call .submit()"]
pub struct UpdateConfigBuilder<'a, C> {
    pub(in crate::config::builder) client: &'a C,
    lock_and_exit: Option<LockAndExit>,
    messages: Vec<MessageSetting>,
    api: Option<ApiConfig>,
    orders: Option<OrdersConfig>,
    accepted_warnings: Vec<ConfigWarning>,
    reset_api_order_sequence: bool,
}

impl<'a, C> UpdateConfigBuilder<'a, C> {
    pub(in crate::config) fn new(client: &'a C) -> Self {
        Self {
            client,
            lock_and_exit: None,
            messages: Vec::new(),
            api: None,
            orders: None,
            accepted_warnings: Vec::new(),
            reset_api_order_sequence: false,
        }
    }

    /// Edit the API-level configuration (precautions and/or settings).
    pub fn api(mut self, api: ApiConfig) -> Self {
        self.api = Some(api);
        self
    }

    /// Edit the order-handling configuration.
    pub fn orders(mut self, orders: OrdersConfig) -> Self {
        self.orders = Some(orders);
        self
    }

    /// Edit the lock-and-exit (auto-logoff) settings.
    pub fn lock_and_exit(mut self, lock_and_exit: LockAndExit) -> Self {
        self.lock_and_exit = Some(lock_and_exit);
        self
    }

    /// Add a message-prompt edit. Call repeatedly to edit multiple prompts.
    pub fn message(mut self, message: MessageSetting) -> Self {
        self.messages.push(message);
        self
    }

    /// Acknowledge a warning from a prior [`UpdateConfigResponse`](crate::config::UpdateConfigResponse)
    /// so the gateway applies the edit on re-submission. Call once per warning.
    pub fn accept_warning(mut self, warning: ConfigWarning) -> Self {
        self.accepted_warnings.push(warning);
        self
    }

    /// Request that the gateway reset the API order-id sequence.
    pub fn reset_api_order_sequence(mut self) -> Self {
        self.reset_api_order_sequence = true;
        self
    }

    /// Assemble the wire request from the accumulated edits.
    pub(in crate::config) fn to_proto(&self, request_id: i32) -> proto::UpdateConfigRequest {
        proto::UpdateConfigRequest {
            req_id: Some(request_id),
            lock_and_exit: self.lock_and_exit.as_ref().map(encoders::to_proto_lock_and_exit),
            messages: self.messages.iter().map(encoders::to_proto_message).collect(),
            api: self.api.as_ref().map(encoders::to_proto_api),
            orders: self.orders.as_ref().map(encoders::to_proto_orders),
            accepted_warnings: self.accepted_warnings.iter().map(encoders::to_proto_warning).collect(),
            reset_api_order_sequence: self.reset_api_order_sequence.then_some(true),
        }
    }
}
