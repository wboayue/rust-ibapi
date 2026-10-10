use super::*;

fn account(name: &str) -> AccountId {
    AccountId(name.to_string())
}

#[test]
fn account_updates_slot_follows_the_account_data_count() {
    let mut counts = SharedCounts::default();
    assert!(counts.check_account_updates(Some(&account("DU1"))).is_ok(), "empty slot refused");

    let first = counts.subscribe(OutgoingMessages::RequestAccountData, Some(&account("DU1")));
    let second = counts.subscribe(OutgoingMessages::RequestAccountData, Some(&account("DU1")));
    assert!(counts.check_account_updates(Some(&account("DU1"))).is_ok(), "same account refused");
    assert!(matches!(
        counts.check_account_updates(Some(&account("DU2"))),
        Err(Error::AccountUpdatesInUse { .. })
    ));

    // Another type's last subscription ending leaves the slot alone.
    let positions = counts.subscribe(OutgoingMessages::RequestPositions, None);
    assert!(counts.unsubscribe(positions));
    assert_eq!(counts.account_updates(), Some(&account("DU1")));

    assert!(!counts.unsubscribe(first));
    assert_eq!(counts.account_updates(), Some(&account("DU1")));
    assert!(counts.unsubscribe(second));
    assert_eq!(counts.account_updates(), None);
    assert!(counts.check_account_updates(Some(&account("DU2"))).is_ok());
}

#[test]
fn reset_frees_the_account_updates_slot() {
    let mut counts = SharedCounts::default();
    let stale = counts.subscribe(OutgoingMessages::RequestAccountData, Some(&account("DU1")));
    counts.reset();
    assert_eq!(counts.account_updates(), None);

    counts.subscribe(OutgoingMessages::RequestAccountData, Some(&account("DU2")));
    assert!(!counts.unsubscribe(stale), "stale ticket released the new session");
    assert_eq!(counts.account_updates(), Some(&account("DU2")));
}

// ---- BufferBound::for_stream -----------------------------------------------

#[test]
fn for_stream_validates_the_limit_and_takes_the_end_from_the_decoder() {
    use crate::contracts::{ContractDetails, OptionChain};
    use crate::messages::IncomingMessages;

    assert!(BufferBound::for_stream::<ContractDetails>(None).unwrap().is_none());
    for limit in [0, MAX_BUFFER_LIMIT + 1] {
        assert!(
            matches!(BufferBound::for_stream::<ContractDetails>(Some(limit)), Err(Error::InvalidArgument(_))),
            "limit {limit} must be rejected"
        );
    }
    for limit in [1, MAX_BUFFER_LIMIT] {
        let bound = BufferBound::for_stream::<ContractDetails>(Some(limit)).unwrap().unwrap();
        assert_eq!(bound.limit, limit);
        assert_eq!(bound.end, IncomingMessages::ContractDataEnd);
    }
    let bound = BufferBound::for_stream::<OptionChain>(Some(8)).unwrap().unwrap();
    assert_eq!(bound.end, IncomingMessages::SecurityDefinitionOptionParameterEnd);
}

#[test]
#[should_panic(expected = "end marker")]
fn for_stream_refuses_a_stream_without_an_end_marker() {
    // `HistoricalDataEnd` is a data item on this stream, not terminal.
    let _ = BufferBound::for_stream::<crate::market_data::historical::HistoricalBarUpdate>(Some(8));
}
