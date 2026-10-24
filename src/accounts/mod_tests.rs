use super::*;

#[test]
fn ledger_prefix_split() {
    let cases = [
        ("$LEDGER-AccruedCash", true, "AccruedCash"),
        ("AccruedCash", false, "AccruedCash"),
        ("$LEDGER:USD", false, "$LEDGER:USD"),
        ("$ledger-AccruedCash", false, "$ledger-AccruedCash"),
        ("$LEDGER-", true, ""),
        ("", false, ""),
    ];
    for (key, per_currency, base) in cases {
        let value = AccountValue {
            key: key.to_string(),
            ..Default::default()
        };
        assert_eq!(value.is_per_currency(), per_currency, "{key}");
        assert_eq!(value.base_key(), base, "{key}");

        let multi = AccountMultiValue {
            key: key.to_string(),
            ..Default::default()
        };
        assert_eq!(multi.is_per_currency(), per_currency, "{key}");
        assert_eq!(multi.base_key(), base, "{key}");
    }
}
