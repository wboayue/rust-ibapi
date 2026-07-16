//! Builders for configuration request and response messages.

use super::ResponseProtoEncoder;
use crate::common::test_utils::helpers::constants::TEST_REQ_ID_FIRST;
use crate::messages::OutgoingMessages;
use crate::proto;

single_req_id_request_builder!(ConfigRequestBuilder, ConfigRequest, OutgoingMessages::ReqConfig);

/// Convenience constructor for the config request builder.
pub fn config_request() -> ConfigRequestBuilder {
    ConfigRequestBuilder::default()
}

/// Field-minimal builder for `proto::ConfigResponse`. Setters populate a
/// representative field in each nested group so decoder tests can assert the
/// proto→domain conversion end to end.
#[derive(Clone, Debug)]
pub struct ConfigResponseBuilder {
    pub request_id: i32,
    inner: proto::ConfigResponse,
}

impl Default for ConfigResponseBuilder {
    fn default() -> Self {
        Self {
            request_id: TEST_REQ_ID_FIRST,
            inner: proto::ConfigResponse::default(),
        }
    }
}

impl ConfigResponseBuilder {
    pub fn request_id(mut self, v: i32) -> Self {
        self.request_id = v;
        self
    }

    pub fn auto_logoff_time(mut self, v: impl Into<String>) -> Self {
        self.inner.lock_and_exit.get_or_insert_with(Default::default).auto_logoff_time = Some(v.into());
        self
    }

    pub fn message(mut self, id: i32, title: impl Into<String>, enabled: bool) -> Self {
        self.inner.messages.push(proto::MessageConfig {
            id: Some(id),
            title: Some(title.into()),
            enabled: Some(enabled),
            ..Default::default()
        });
        self
    }

    pub fn read_only_api(mut self, v: bool) -> Self {
        self.settings().read_only_api = Some(v);
        self
    }

    pub fn socket_port(mut self, v: i32) -> Self {
        self.settings().socket_port = Some(v);
        self
    }

    pub fn trusted_ip(mut self, v: impl Into<String>) -> Self {
        self.settings().trusted_i_ps.push(v.into());
        self
    }

    pub fn bypass_bond_warning(mut self, v: bool) -> Self {
        self.inner
            .api
            .get_or_insert_with(Default::default)
            .precautions
            .get_or_insert_with(Default::default)
            .bypass_bond_warning = Some(v);
        self
    }

    pub fn seek_price_improvement(mut self, v: bool) -> Self {
        self.inner
            .orders
            .get_or_insert_with(Default::default)
            .smart_routing
            .get_or_insert_with(Default::default)
            .seek_price_improvement = Some(v);
        self
    }

    fn settings(&mut self) -> &mut proto::ApiSettingsConfig {
        self.inner
            .api
            .get_or_insert_with(Default::default)
            .settings
            .get_or_insert_with(Default::default)
    }
}

impl ResponseProtoEncoder for ConfigResponseBuilder {
    type Proto = proto::ConfigResponse;

    fn to_proto(&self) -> Self::Proto {
        let mut proto = self.inner.clone();
        proto.req_id = Some(self.request_id);
        proto
    }
}

/// Convenience constructor for the config response builder.
pub fn config_response() -> ConfigResponseBuilder {
    ConfigResponseBuilder::default()
}
