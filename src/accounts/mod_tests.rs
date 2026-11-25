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

/// `impl_wire_enum!(.., integer)`: text form through `Display` / `FromStr` / `ToField`, serde as
/// the bare code, and an unlisted code rejected.
fn check_integer_wire_enum<T>(table: &[(T, i32)], unlisted: i32)
where
    T: std::fmt::Debug + PartialEq + std::str::FromStr<Err = Error> + crate::ToField + serde::Serialize + serde::de::DeserializeOwned,
{
    use crate::common::test_utils::wire_enum::check_serde_form;

    for (variant, code) in table {
        assert_eq!(variant.to_field(), code.to_string(), "ToField for {variant:?}");
        assert_eq!(&code.to_string().parse::<T>().unwrap(), variant, "FromStr({code})");
        check_serde_form(variant, serde_json::json!(code));
    }
    assert!(serde_json::from_value::<T>(serde_json::json!(unlisted)).is_err(), "code {unlisted}");
}

#[test]
fn fa_data_type_round_trip() {
    check_integer_wire_enum(&[(FaDataType::Groups, 1), (FaDataType::AccountAliases, 3)], 2);
}

#[test]
fn server_log_level_round_trip() {
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
