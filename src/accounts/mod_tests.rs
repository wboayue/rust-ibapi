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
