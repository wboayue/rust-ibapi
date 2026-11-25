use super::*;

#[test]
fn ledger_prefix_split() {
    // (key, has_ledger_prefix, base_key)
    let cases = [
        ("$LEDGER-AccruedCash", true, "AccruedCash"),
        ("AccruedCash", false, "AccruedCash"),
        ("$LEDGER:USD", false, "$LEDGER:USD"),
        ("$ledger-AccruedCash", false, "$ledger-AccruedCash"),
        ("$LEDGER-", true, ""),
        ("", false, ""),
    ];
    for (key, prefixed, base) in cases {
        let value = AccountMultiValue {
            key: key.to_string(),
            ..Default::default()
        };
        assert_eq!(value.has_ledger_prefix(), prefixed, "{key}");
        assert_eq!(value.base_key(), base, "{key}");
    }
}

#[test]
fn fa_data_type_round_trip() {
    use crate::common::test_utils::wire_enum::check_integer_wire_enum;

    check_integer_wire_enum(&[(FaDataType::Groups, 1), (FaDataType::AccountAliases, 3)], 2);
}

#[test]
fn server_log_level_round_trip() {
    use crate::common::test_utils::wire_enum::check_integer_wire_enum;

    check_integer_wire_enum(
        &[
            (ServerLogLevel::System, 1),
            (ServerLogLevel::Error, 2),
            (ServerLogLevel::Warning, 3),
            (ServerLogLevel::Info, 4),
            (ServerLogLevel::Detail, 5),
        ],
        0,
    );
}
