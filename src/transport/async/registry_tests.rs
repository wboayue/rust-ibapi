use super::*;
use crate::client::ids::RequestId;
use crate::transport::common::Lease;
use crate::transport::BufferBound;

/// A route, its receiver, and the lease that keeps it live.
fn route() -> (Route, broadcast::Receiver<RoutedItem>, Lease) {
    let (sender, receiver) = broadcast::channel(8);
    let lease = Lease::new();
    (Route::unbounded(sender, lease.downgrade()), receiver, lease)
}

fn items(receiver: &mut broadcast::Receiver<RoutedItem>) -> Vec<RoutedItem> {
    std::iter::from_fn(|| receiver.try_recv().ok()).collect()
}

fn is_reset(item: &RoutedItem) -> bool {
    matches!(item, RoutedItem::Error(Error::ConnectionReset))
}

#[test]
fn fail_all_delivers_then_clears() {
    let routes = SenderHash::new();
    let (route, mut receiver, _lease) = route();
    routes.insert(RequestId::nth(1), route);

    routes.fail_all(|| Error::ConnectionReset.into());

    assert!(routes.is_empty());
    let items = items(&mut receiver);
    assert!(items.len() == 1 && is_reset(&items[0]), "{items:?}");
    assert!(matches!(receiver.try_recv(), Err(broadcast::error::TryRecvError::Closed)));
}

/// A closed bounded route's stream already ended; failing it again would put
/// a second terminal item behind the first.
#[test]
fn fail_all_skips_a_closed_bounded_route() {
    let routes = SenderHash::new();
    let (sender, mut receiver) = broadcast::channel(4);
    let bound = BufferBound {
        limit: 1,
        end: IncomingMessages::ContractDataEnd,
    };
    let lease = Lease::new();
    let route = Route::bounded(sender, lease.downgrade(), bound, Arc::new(AtomicUsize::new(0)));
    routes.insert(RequestId::nth(1), route);
    // An error is terminal: it closes the route.
    routes.deliver(&RequestId::nth(1), RoutedItem::Error(Error::Cancelled)).unwrap();

    routes.fail_all(|| Error::ConnectionReset.into());

    assert!(!items(&mut receiver).iter().any(is_reset), "closed route was failed again");
}

#[test]
fn deliver_hands_the_item_back_when_unrouted() {
    let routes = SenderHash::<RequestId>::new();
    let item = routes.deliver(&RequestId::nth(1), RoutedItem::Error(Error::Cancelled));
    assert!(matches!(item, Err(RoutedItem::Error(Error::Cancelled))));
}

#[test]
fn deliver_aliased_aliases_the_route() {
    let routes = SenderHash::new();
    let aliases = SenderHash::<String>::new();
    let (route, mut receiver, lease) = route();
    routes.insert(RequestId::nth(1), route);
    let alias = "exec-1".to_string();

    routes
        .deliver_aliased(&RequestId::nth(1), RoutedItem::Error(Error::Cancelled), Some(&alias), &aliases)
        .unwrap();
    aliases.deliver(&alias, Error::ConnectionReset.into()).unwrap();

    let items = items(&mut receiver);
    assert!(items.len() == 2 && is_reset(&items[1]), "{items:?}");
    assert!(
        aliases.with_route(&alias, |route| route.lease.is(&lease.downgrade())).unwrap(),
        "alias holds another lease"
    );
}

#[test]
fn deliver_aliased_without_alias_registers_none() {
    let routes = SenderHash::new();
    let aliases = SenderHash::<String>::new();
    let (route, mut receiver, _lease) = route();
    routes.insert(RequestId::nth(1), route);

    routes
        .deliver_aliased(&RequestId::nth(1), RoutedItem::Error(Error::Cancelled), None, &aliases)
        .unwrap();

    assert_eq!(items(&mut receiver).len(), 1);
    assert!(aliases.is_empty());
}

#[test]
fn deliver_aliased_hands_the_item_back_when_unrouted() {
    let routes = SenderHash::<RequestId>::new();
    let aliases = SenderHash::<String>::new();

    let item = routes.deliver_aliased(
        &RequestId::nth(1),
        RoutedItem::Error(Error::Cancelled),
        Some(&"exec-1".to_string()),
        &aliases,
    );

    assert!(matches!(item, Err(RoutedItem::Error(Error::Cancelled))));
    assert!(aliases.is_empty(), "unrouted item was aliased");
}

#[test]
fn remove_if_same_spares_a_replacement() {
    let routes = SenderHash::new();
    let (stale, _stale_receiver, stale_lease) = route();
    routes.insert(RequestId::nth(1), stale);
    let (replacement, _receiver, _lease) = route();
    routes.insert(RequestId::nth(1), replacement);

    routes.remove_if_same(RequestId::nth(1), &stale_lease.downgrade());

    assert!(routes.contains(&RequestId::nth(1)));
}

#[test]
fn release_spares_a_live_route() {
    let routes = SenderHash::new();
    let (route, _receiver, lease) = route();
    let lease_ref = lease.downgrade();
    let clone = lease.clone();
    routes.insert(RequestId::nth(1), route);

    drop(lease);
    routes.release(RequestId::nth(1), &lease_ref, "request");
    assert!(routes.contains(&RequestId::nth(1)), "route removed while a clone holds the lease");

    drop(clone);
    routes.release(RequestId::nth(1), &lease_ref, "request");
    assert!(!routes.contains(&RequestId::nth(1)), "dead route kept");
}

/// A stale signal finds a dead replacement under its key: not its own, so the
/// replacement's signal is left to remove it.
#[test]
fn release_spares_another_leases_route() {
    let routes = SenderHash::new();
    let (_, _, stale) = route();
    let (replacement, _receiver, lease) = route();
    routes.insert(RequestId::nth(1), replacement);
    drop(lease);

    routes.release(RequestId::nth(1), &stale.downgrade(), "request");

    assert!(routes.contains(&RequestId::nth(1)));
}

#[test]
fn prune_dead_drops_only_dead_routes() {
    let routes = SenderHash::new();
    let (live, _receiver, _lease) = route();
    let (dead, _receiver, _) = route();
    routes.insert("live".to_string(), live);
    routes.insert("dead".to_string(), dead);

    routes.prune_dead();

    assert!(routes.contains(&"live".to_string()));
    assert!(!routes.contains(&"dead".to_string()));
}

async fn subscribe(channels: &SharedChannels, request: OutgoingMessages) -> broadcast::Receiver<RoutedItem> {
    channels.subscribe(request, None, || async { Ok(()) }).await.unwrap().0
}

#[tokio::test]
async fn fail_one_shot_channels_spares_streaming_channels() {
    let channels = SharedChannels::new(|_| 8);
    let mut current_time = subscribe(&channels, OutgoingMessages::RequestCurrentTime).await;
    let mut positions = subscribe(&channels, OutgoingMessages::RequestPositions).await;

    channels.fail_one_shot_channels(|| Error::ConnectionReset.into());

    assert_eq!(items(&mut current_time).len(), 1);
    assert!(items(&mut positions).is_empty(), "streaming channel failed");
}

#[tokio::test]
async fn subscribe_counts_only_a_written_request() {
    let channels = SharedChannels::new(|_| 8);
    let failed = channels
        .subscribe(OutgoingMessages::RequestPositions, None, || async { Err(Error::ConnectionReset) })
        .await;
    assert!(matches!(failed, Err(Error::ConnectionReset)));
    assert_eq!(channels.live(OutgoingMessages::RequestPositions).await, 0);

    subscribe(&channels, OutgoingMessages::RequestPositions).await;
    assert_eq!(channels.live(OutgoingMessages::RequestPositions).await, 1);
}

#[tokio::test]
async fn close_fails_then_ends_every_channel() {
    let channels = SharedChannels::new(|_| 8);
    let mut positions = subscribe(&channels, OutgoingMessages::RequestPositions).await;

    channels.close(|| Error::Shutdown.into());

    assert!(matches!(positions.try_recv(), Ok(RoutedItem::Error(Error::Shutdown))));
    assert!(matches!(positions.try_recv(), Err(broadcast::error::TryRecvError::Closed)));
    let refused = channels.subscribe(OutgoingMessages::RequestPositions, None, || async { Ok(()) }).await;
    assert!(matches!(refused, Err(Error::InvalidArgument(_))));
}

/// Each shared channel takes its request type's capacity, allocated on the
/// first subscription; sends before then are no-ops.
#[tokio::test]
async fn shared_channels_take_per_type_capacity_lazily() {
    let channels = SharedChannels::new(|request| if request == OutgoingMessages::RequestCompletedOrders { 4 } else { 1 });
    channels.notify_all(|| Error::Cancelled.into());

    let mut completed = subscribe(&channels, OutgoingMessages::RequestCompletedOrders).await;
    let mut positions = subscribe(&channels, OutgoingMessages::RequestPositions).await;
    for _ in 0..2 {
        channels.notify_all(|| Error::Cancelled.into());
    }

    assert_eq!(items(&mut completed).len(), 2, "capacity-4 channel lost an item");
    assert!(matches!(positions.try_recv(), Err(broadcast::error::TryRecvError::Lagged(1))));
}
