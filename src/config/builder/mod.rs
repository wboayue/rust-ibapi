//! Fluent builder for the `update_config` write path.

pub(crate) mod update_config_builder;

#[cfg(feature = "sync")]
mod sync_impl;

#[cfg(feature = "async")]
mod async_impl;

pub use update_config_builder::UpdateConfigBuilder;
