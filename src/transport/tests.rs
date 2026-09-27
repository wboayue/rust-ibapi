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
