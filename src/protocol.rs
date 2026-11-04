//! Server version gates for features above the connection floor.
//!
//! The handshake refuses servers below version 213 (`PROTOBUF_REST_MESSAGES_3`),
//! so a feature at or below it is always supported and needs no gate. A feature
//! above the floor gets a [`ProtocolFeature`] and a [`check_version`] call on
//! the line before its request; a caller sees `Error::ServerVersion`.

use crate::errors::Error;
use crate::server_versions;

/// A feature that requires a minimum server version.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProtocolFeature {
    /// The name of the feature, for error messages.
    pub(crate) name: &'static str,
    /// The minimum server version required.
    pub(crate) min_version: i32,
}

impl ProtocolFeature {
    pub(crate) const fn new(name: &'static str, min_version: i32) -> Self {
        Self { name, min_version }
    }
}

/// Request gates. Order-field gates are local to `orders/common/verify.rs`,
/// beside `verify_order`.
pub(crate) struct Features;

impl Features {
    /// Cancelling in-flight contract data and historical ticks requests.
    pub(crate) const CANCEL_CONTRACT_DATA: ProtocolFeature = ProtocolFeature::new("cancel contract data", server_versions::CANCEL_CONTRACT_DATA);
    /// Reading TWS/Gateway configuration via reqConfig.
    pub(crate) const CONFIG: ProtocolFeature = ProtocolFeature::new("configuration", server_versions::CONFIG);
    /// Writing TWS/Gateway configuration via updateConfig.
    pub(crate) const UPDATE_CONFIG: ProtocolFeature = ProtocolFeature::new("update configuration", server_versions::UPDATE_CONFIG);
}

/// `Ok(())` if `server_version` supports `feature`, else
/// `Error::ServerVersion(required, actual, name)`.
pub(crate) fn check_version(server_version: i32, feature: ProtocolFeature) -> Result<(), Error> {
    if server_version < feature.min_version {
        Err(Error::ServerVersion(feature.min_version, server_version, feature.name.to_string()))
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
