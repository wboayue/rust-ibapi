use super::*;

#[test]
fn test_to_proto_settings_maps_renamed_fields() {
    // The prost-generated names diverge from the domain names; this guards the
    // hand-written mapping for those fields specifically.
    let settings = ApiSettings {
        prepare_daily_pnl: Some(true),
        show_forex_data_in_1_10_pips: Some(true),
        allow_forex_trading_in_1_10_pips: Some(false),
        socket_port: Some(7497),
        trusted_ips: vec!["127.0.0.1".to_string()],
        logging_level: Some("error".to_string()),
        ..Default::default()
    };

    let p = to_proto_settings(&settings);

    assert_eq!(p.prepare_daily_pn_l, Some(true));
    assert_eq!(p.show_forex_data_in1_10pips, Some(true));
    assert_eq!(p.allow_forex_trading_in1_10pips, Some(false));
    assert_eq!(p.socket_port, Some(7497));
    assert_eq!(p.trusted_i_ps, vec!["127.0.0.1".to_string()]);
    assert_eq!(p.logging_level, Some("error".to_string()));
}

#[test]
fn test_to_proto_api_nests_precautions_and_settings() {
    let api = ApiConfig {
        precautions: Some(ApiPrecautions {
            bypass_bond_warning: Some(true),
            ..Default::default()
        }),
        settings: Some(ApiSettings {
            read_only_api: Some(true),
            ..Default::default()
        }),
    };

    let p = to_proto_api(&api);

    assert_eq!(p.precautions.unwrap().bypass_bond_warning, Some(true));
    assert_eq!(p.settings.unwrap().read_only_api, Some(true));
}

#[test]
fn test_to_proto_orders_and_leaf_converters() {
    let orders = OrdersConfig {
        smart_routing: Some(OrdersSmartRouting {
            seek_price_improvement: Some(true),
            default_algorithm: Some("Adaptive".to_string()),
            ..Default::default()
        }),
    };
    let p = to_proto_orders(&orders);
    let sr = p.smart_routing.unwrap();
    assert_eq!(sr.seek_price_improvement, Some(true));
    assert_eq!(sr.default_algorithm, Some("Adaptive".to_string()));

    let le = to_proto_lock_and_exit(&LockAndExit {
        auto_logoff_time: Some("23:59".to_string()),
        ..Default::default()
    });
    assert_eq!(le.auto_logoff_time, Some("23:59".to_string()));

    let msg = to_proto_message(&MessageSetting {
        id: Some(131),
        enabled: Some(false),
        ..Default::default()
    });
    assert_eq!(msg.id, Some(131));
    assert_eq!(msg.enabled, Some(false));

    let warn = to_proto_warning(&ConfigWarning {
        message_id: Some(131),
        ..Default::default()
    });
    assert_eq!(warn.message_id, Some(131));
}

#[test]
fn test_encode_update_config_frames_message_id() {
    let request = proto::UpdateConfigRequest {
        req_id: Some(42),
        reset_api_order_sequence: Some(true),
        ..Default::default()
    };

    let bytes = encode_update_config(&request).unwrap();

    // 4-byte msg-id header carries OutgoingMessages::UpdateConfig, payload decodes back.
    let payload = &bytes[4..];
    let decoded = proto::UpdateConfigRequest::decode(payload).unwrap();
    assert_eq!(decoded.req_id, Some(42));
    assert_eq!(decoded.reset_api_order_sequence, Some(true));
}
