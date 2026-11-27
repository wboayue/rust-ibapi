use super::*;
use crate::proto;
use prost::Message;

// The builders only borrow the client until a feature-gated `subscribe`; `()` stands in for it.
const CLIENT: &() = &();

fn decode<M: Message + Default>(bytes: &[u8]) -> M {
    M::decode(&bytes[4..]).expect("proto body")
}

#[test]
fn positions_multi_sends_only_the_filters_set() {
    let cases = [
        ("no filter", PositionsMultiBuilder::new(CLIENT), None, None),
        ("account", PositionsMultiBuilder::new(CLIENT).account("DU1"), Some("DU1"), None),
        ("model code", PositionsMultiBuilder::new(CLIENT).model_code("M1"), None, Some("M1")),
        (
            "both",
            PositionsMultiBuilder::new(CLIENT).model_code("M1").account("DU1"),
            Some("DU1"),
            Some("M1"),
        ),
    ];
    for (label, builder, account, model_code) in cases {
        let request: proto::PositionsMultiRequest = decode(&builder.encode(9000).unwrap());
        assert_eq!(request.req_id, Some(9000), "{label}");
        assert_eq!(request.account.as_deref(), account, "{label}");
        assert_eq!(request.model_code.as_deref(), model_code, "{label}");
    }
}

#[test]
fn account_updates_multi_sends_filters_and_ledger_only() {
    let cases = [
        (
            "no filter, ledger only",
            AccountUpdatesMultiBuilder::new(CLIENT, LedgerOnly::Yes),
            None,
            None,
            Some(true),
        ),
        (
            "account and model, all keys",
            AccountUpdatesMultiBuilder::new(CLIENT, LedgerOnly::No).account("DU1").model_code("M1"),
            Some("DU1"),
            Some("M1"),
            None,
        ),
    ];
    for (label, builder, account, model_code, ledger_and_nlv) in cases {
        let request: proto::AccountUpdatesMultiRequest = decode(&builder.encode(9000).unwrap());
        assert_eq!(request.req_id, Some(9000), "{label}");
        assert_eq!(request.account.as_deref(), account, "{label}");
        assert_eq!(request.model_code.as_deref(), model_code, "{label}");
        assert_eq!(request.ledger_and_nlv, ledger_and_nlv, "{label}");
    }
}

#[test]
fn pnl_sends_account_and_optional_model_code() {
    let cases = [
        ("account only", PnLBuilder::new(CLIENT, "DU1"), None),
        ("with model code", PnLBuilder::new(CLIENT, "DU1").model_code("M1"), Some("M1")),
    ];
    for (label, builder, model_code) in cases {
        let request: proto::PnLRequest = decode(&builder.encode(9000).unwrap());
        assert_eq!(request.req_id, Some(9000), "{label}");
        assert_eq!(request.account.as_deref(), Some("DU1"), "{label}");
        assert_eq!(request.model_code.as_deref(), model_code, "{label}");
    }
}

#[test]
fn pnl_single_sends_account_contract_and_optional_model_code() {
    let cases = [
        ("no model code", PnLSingleBuilder::new(CLIENT, "DU1", 265598), None),
        (
            "with model code",
            PnLSingleBuilder::new(CLIENT, "DU1", 265598).model_code("M1"),
            Some("M1"),
        ),
    ];
    for (label, builder, model_code) in cases {
        let request: proto::PnLSingleRequest = decode(&builder.encode(9000).unwrap());
        assert_eq!(request.req_id, Some(9000), "{label}");
        assert_eq!(request.account.as_deref(), Some("DU1"), "{label}");
        assert_eq!(request.con_id, Some(265598), "{label}");
        assert_eq!(request.model_code.as_deref(), model_code, "{label}");
    }
}
