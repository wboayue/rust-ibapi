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
    use crate::market_data::historical::HistoricalDataFrame;
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
        assert_eq!(bound.end, &[IncomingMessages::ContractDataEnd]);
    }
    let bound = BufferBound::for_stream::<OptionChain>(Some(8)).unwrap().unwrap();
    assert_eq!(bound.end, &[IncomingMessages::SecurityDefinitionOptionParameterEnd]);
    // `HistoricalDataEnd` is a data item on this stream, not an end marker.
    let bound = BufferBound::for_stream::<HistoricalDataFrame>(Some(8)).unwrap().unwrap();
    assert!(bound.end.is_empty());
}

// ---- BoundState::admit -----------------------------------------------------

fn response(kind: crate::messages::IncomingMessages) -> RoutedItem {
    RoutedItem::Response(ResponseMessage::from(&format!("{}\0", kind as i32)))
}

#[test]
fn admit_without_an_end_marker_closes_only_on_error_or_overflow() {
    use crate::messages::IncomingMessages::{HistoricalData, HistoricalDataEnd};

    let state = BoundState::new(BufferBound { limit: 2, end: &[] });
    assert_eq!(state.admit(&response(HistoricalData), 0), Admit::Deliver);
    assert_eq!(state.admit(&response(HistoricalDataEnd), 1), Admit::Deliver);
    assert!(!state.closed(), "no response is terminal without an end marker");
    assert_eq!(state.admit(&response(HistoricalData), 2), Admit::Overflow);
    assert_eq!(state.admit(&response(HistoricalData), 0), Admit::Discard);

    let state = BoundState::new(BufferBound { limit: 2, end: &[] });
    assert_eq!(state.admit(&Error::ConnectionReset.into(), 0), Admit::Deliver);
    assert!(state.closed(), "an error is terminal");
}

#[test]
fn admit_closes_on_any_of_several_end_markers() {
    use crate::messages::IncomingMessages::{CompletedOrdersEnd, OpenOrder, OpenOrderEnd};

    for end in [OpenOrderEnd, CompletedOrdersEnd] {
        let state = BoundState::new(BufferBound {
            limit: 1,
            end: &[OpenOrderEnd, CompletedOrdersEnd],
        });
        assert_eq!(state.admit(&response(OpenOrder), 0), Admit::Deliver);
        // At the cap, an end marker still gets through.
        assert_eq!(state.admit(&response(end), 1), Admit::Deliver);
        assert!(state.closed(), "{end:?} is terminal");
    }
}

/// Requests sharing an end marker share one count, an end marker with
/// nothing outstanding (the `OpenOrderEnd` TWS sends at connect) is ignored,
/// and a skip passes over every frame up to and including its last end marker.
#[test]
fn unanswered_replies_count_by_end_marker() {
    use crate::messages::IncomingMessages as M;
    let mut unanswered = UnansweredReplies::default();

    unanswered.answered(M::OpenOrderEnd);
    assert!(!unanswered.add(OutgoingMessages::RequestOpenOrders).skips(M::OpenOrder));
    assert!(
        !unanswered.add(OutgoingMessages::RequestCurrentTime).skips(M::CurrentTime),
        "one-shots never skip"
    );

    let mut all = unanswered.add(OutgoingMessages::RequestAllOpenOrders);
    let mut completed = unanswered.add(OutgoingMessages::RequestCompletedOrders);
    assert_eq!(unanswered.pending(M::OpenOrderEnd), 2);
    assert!(!completed.skips(M::CompletedOrder), "another end marker's requests counted");

    for frame in [M::OpenOrder, M::OrderStatus, M::OpenOrderEnd] {
        assert!(all.skips(frame), "{frame:?} of the earlier reply delivered");
    }
    assert!(!all.skips(M::OpenOrder), "own reply skipped");

    unanswered.answered(M::OpenOrderEnd);
    unanswered.answered(M::OpenOrderEnd);
    unanswered.answered(M::OpenOrderEnd);
    assert_eq!(unanswered.pending(M::OpenOrderEnd), 0);

    unanswered.add(OutgoingMessages::RequestPositions);
    unanswered.withdraw(OutgoingMessages::RequestPositions);
    assert_eq!(unanswered.pending(M::PositionEnd), 0, "a withdrawn request still counted");

    unanswered.add(OutgoingMessages::RequestPositions);
    unanswered.add(OutgoingMessages::RequestPositions);
    unanswered.cancelled(OutgoingMessages::RequestPositions);
    assert_eq!(unanswered.pending(M::PositionEnd), 0, "a cancel kept a count");
    unanswered.add(OutgoingMessages::RequestAccountData);
    assert_eq!(unanswered.pending(M::AccountDownloadEnd), 1, "account updates are counted");

    unanswered.add(OutgoingMessages::RequestOpenOrders);
    unanswered.clear();
    assert_eq!(unanswered.pending(M::OpenOrderEnd), 0, "clear kept a count");
}

pub(super) fn account_update_dump(value: &str) -> [Vec<u8>; 2] {
    use crate::common::test_utils::helpers::binary_proto;
    use crate::messages::IncomingMessages as M;

    [
        binary_proto(
            M::AccountValue as i32,
            &crate::proto::AccountValue {
                key: Some("NetLiquidation".to_string()),
                value: Some(value.to_string()),
                currency: Some("USD".to_string()),
                account_name: Some("DU1".to_string()),
            },
        ),
        binary_proto(
            M::AccountDownloadEnd as i32,
            &crate::proto::AccountDataEnd {
                account_name: Some("DU1".to_string()),
            },
        ),
    ]
}

pub(super) fn account_update_value(item: crate::subscriptions::SubscriptionItem<crate::accounts::AccountUpdate>) -> Option<String> {
    use crate::accounts::AccountUpdate;
    use crate::subscriptions::SubscriptionItem;

    match item {
        SubscriptionItem::Data(AccountUpdate::AccountValue(value)) => Some(value.value),
        SubscriptionItem::Data(AccountUpdate::End) => None,
        other => panic!("unexpected account update {other:?}"),
    }
}
