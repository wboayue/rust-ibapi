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
    use crate::subscriptions::DecoderContext;

    assert!(BufferBound::for_stream::<ContractDetails>(None, &DecoderContext::default())
        .unwrap()
        .is_none());
    for limit in [0, MAX_BUFFER_LIMIT + 1] {
        assert!(
            matches!(
                BufferBound::for_stream::<ContractDetails>(Some(limit), &DecoderContext::default()),
                Err(Error::InvalidArgument(_))
            ),
            "limit {limit} must be rejected"
        );
    }
    for limit in [1, MAX_BUFFER_LIMIT] {
        let bound = BufferBound::for_stream::<ContractDetails>(Some(limit), &DecoderContext::default())
            .unwrap()
            .unwrap();
        assert_eq!(bound.limit, limit);
        assert_eq!(bound.end, &[IncomingMessages::ContractDataEnd]);
    }
    let bound = BufferBound::for_stream::<OptionChain>(Some(8), &DecoderContext::default())
        .unwrap()
        .unwrap();
    assert_eq!(bound.end, &[IncomingMessages::SecurityDefinitionOptionParameterEnd]);
    // `HistoricalDataEnd` is a data item on this stream, not an end marker.
    let bound = BufferBound::for_stream::<HistoricalDataFrame>(Some(8), &DecoderContext::default())
        .unwrap()
        .unwrap();
    assert!(bound.end.is_empty());
}

// ---- BoundState::admit -----------------------------------------------------

fn response(kind: crate::messages::IncomingMessages) -> RoutedItem {
    RoutedItem::Response(ResponseMessage::from(&format!("{}\0", kind as i32)))
}

#[test]
fn admit_without_an_end_marker_closes_only_on_error_or_overflow() {
    use crate::messages::IncomingMessages::{HistoricalData, HistoricalDataEnd};

    let state = BoundState::new(BufferBound {
        limit: 2,
        end: &[],
        continues: |_, _| false,
        context: crate::subscriptions::DecoderContext::default(),
    });
    assert_eq!(state.admit(&response(HistoricalData), 0), Admit::Deliver);
    assert_eq!(state.admit(&response(HistoricalDataEnd), 1), Admit::Deliver);
    assert!(!state.closed(), "no response is terminal without an end marker");
    assert_eq!(state.admit(&response(HistoricalData), 2), Admit::Overflow);
    assert_eq!(state.admit(&response(HistoricalData), 0), Admit::Discard);

    let state = BoundState::new(BufferBound {
        limit: 2,
        end: &[],
        continues: |_, _| false,
        context: crate::subscriptions::DecoderContext::default(),
    });
    assert_eq!(state.admit(&Error::ConnectionReset.into(), 0), Admit::Deliver);
    assert!(state.closed(), "an error is terminal");
}

/// An error the decoder continues after (10299 on historical bars) is queued like data:
/// the route stays open, and past the cap it overflows. Any other error still closes it.
#[test]
fn admit_keeps_the_route_open_after_an_error_the_decoder_continues() {
    use crate::market_data::historical::HistoricalDataFrame;
    use crate::messages::IncomingMessages::HistoricalData;
    use crate::messages::Notice;
    use crate::subscriptions::DecoderContext;

    let error = |code| RoutedItem::Error(Error::Notice(Notice::synthesized(code, String::new())));
    let bound = || {
        BufferBound::for_stream::<HistoricalDataFrame>(Some(2), &DecoderContext::default())
            .unwrap()
            .unwrap()
    };

    let state = BoundState::new(bound());
    assert_eq!(state.admit(&error(10299), 0), Admit::Deliver);
    assert!(!state.closed(), "10299 does not end a historical request");
    assert_eq!(state.admit(&response(HistoricalData), 1), Admit::Deliver);
    assert_eq!(state.admit(&error(162), 2), Admit::Deliver);
    assert!(state.closed(), "162 does");

    let state = BoundState::new(bound());
    assert_eq!(
        state.admit(&error(10299), 2),
        Admit::Overflow,
        "a continuing error counts against the cap"
    );
}

#[test]
fn admit_closes_on_any_of_several_end_markers() {
    use crate::messages::IncomingMessages::{CompletedOrdersEnd, OpenOrder, OpenOrderEnd};

    for end in [OpenOrderEnd, CompletedOrdersEnd] {
        let state = BoundState::new(BufferBound {
            limit: 1,
            end: &[OpenOrderEnd, CompletedOrdersEnd],
            continues: |_, _| false,
            context: crate::subscriptions::DecoderContext::default(),
        });
        assert_eq!(state.admit(&response(OpenOrder), 0), Admit::Deliver);
        // At the cap, an end marker still gets through.
        assert_eq!(state.admit(&response(end), 1), Admit::Deliver);
        assert!(state.closed(), "{end:?} is terminal");
    }
}
