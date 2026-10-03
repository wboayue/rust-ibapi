use super::*;
use std::sync::Arc;
use std::thread;

#[test]
fn test_id_generator_basic() {
    let gen = IdGenerator::new(100);
    assert_eq!(gen.current(), 100);
    assert_eq!(gen.next(), 100);
    assert_eq!(gen.next(), 101);
    assert_eq!(gen.next(), 102);
    assert_eq!(gen.current(), 103);
}

#[test]
fn test_id_generator_raise() {
    let gen = IdGenerator::new(100);
    assert_eq!(gen.next(), 100);
    gen.raise(200);
    assert_eq!(gen.next(), 200);
    assert_eq!(gen.next(), 201);

    // A server value at or below the allocated high-water mark must not
    // reissue allocated IDs: the server cannot know about ID 100 until its
    // order is transmitted, yet `next` must never emit 100 again.
    gen.raise(100);
    assert_eq!(gen.next(), 202);
    assert_eq!(gen.next(), 203);
}

/// Concurrent `next` and `raise` must never reissue an ID. A store-based
/// server update racing `fetch_add` rewinds the counter and duplicates
/// allocations; `fetch_max` makes every interleaving safe.
#[test]
fn test_id_generator_concurrent_next_and_raise() {
    let gen = Arc::new(IdGenerator::new(0));

    let raiser = {
        let gen = Arc::clone(&gen);
        thread::spawn(move || {
            for value in 0..20_000 {
                gen.raise(value % 500);
            }
        })
    };

    let mut allocators = vec![];
    for _ in 0..4 {
        let gen = Arc::clone(&gen);
        allocators.push(thread::spawn(move || (0..2_000).map(|_| gen.next()).collect::<Vec<i32>>()));
    }

    raiser.join().unwrap();
    let mut all_ids = vec![];
    for allocator in allocators {
        all_ids.extend(allocator.join().unwrap());
    }

    all_ids.sort();
    let has_duplicate = all_ids.windows(2).any(|pair| pair[0] == pair[1]);
    assert!(!has_duplicate, "concurrent raise reissued an order ID");
}

#[test]
fn test_id_generator_thread_safe() {
    let gen = Arc::new(IdGenerator::new(0));
    let mut handles = vec![];

    // Spawn 10 threads, each getting 100 IDs
    for _ in 0..10 {
        let gen_clone = Arc::clone(&gen);
        let handle = thread::spawn(move || {
            let mut ids = vec![];
            for _ in 0..100 {
                ids.push(gen_clone.next());
            }
            ids
        });
        handles.push(handle);
    }

    // Collect all IDs
    let mut all_ids = vec![];
    for handle in handles {
        all_ids.extend(handle.join().unwrap());
    }

    // Check that we have 1000 unique IDs from 0 to 999
    all_ids.sort();
    assert_eq!(all_ids.len(), 1000);
    for (i, id) in all_ids.iter().enumerate() {
        assert_eq!(*id, i as i32);
    }
}

#[test]
fn test_request_id_generator() {
    let gen = IdGenerator::new_request_id_generator();
    assert_eq!(gen.current(), REQUEST_ID_FLOOR);
    assert_eq!(gen.next(), REQUEST_ID_FLOOR);
    assert_eq!(gen.next(), REQUEST_ID_FLOOR + 1);
}

#[test]
fn test_next_up_to_stops_at_max() {
    let gen = IdGenerator::new(9);
    assert_eq!(gen.next_up_to(10), Some(9));
    assert_eq!(gen.next_up_to(10), Some(10));
    assert_eq!(gen.next_up_to(10), None);
    assert_eq!(gen.current(), 11, "a refused id leaves the counter put");
}

#[test]
fn test_client_id_manager() {
    let manager = ClientIdManager::new(50).unwrap();

    // Test request IDs
    assert_eq!(manager.current_request_id(), REQUEST_ID_FLOOR);
    assert_eq!(manager.next_request_id(), RequestId::nth(0));
    assert_eq!(manager.next_request_id(), RequestId::nth(1));

    // Test order IDs
    assert_eq!(manager.current_order_id(), 50);
    assert_eq!(manager.next_order_id(), OrderId::from(50));
    assert_eq!(manager.next_order_id(), OrderId::from(51));

    // Test order ID raise
    manager.raise_order_id(OrderId::from(100));
    assert_eq!(manager.next_order_id(), OrderId::from(100));
    assert_eq!(manager.next_order_id(), OrderId::from(101));

    // Raising below the allocated high-water mark must not reissue IDs.
    manager.raise_order_id(OrderId::from(100));
    assert_eq!(manager.next_order_id(), OrderId::from(102));
}

/// A seed in the request range would put every order the session places on
/// request ids (#789).
#[test]
fn test_client_id_manager_rejects_seed_in_request_range() {
    assert!(ClientIdManager::new(REQUEST_ID_FLOOR - 1).is_ok());
    let err = ClientIdManager::new(REQUEST_ID_FLOOR).unwrap_err();
    assert!(
        matches!(&err, Error::ConnectionRejected(m) if m.contains(&REQUEST_ID_FLOOR.to_string())),
        "{err:?}"
    );
}

/// Request ids never reach the order range, and stop before `i32::MAX`, whose
/// error frames TWS sends without an id.
#[test]
fn test_request_ids_stop_at_ceiling() {
    let manager = ClientIdManager::new(0).unwrap();
    manager.request_ids.raise(REQUEST_ID_CEILING);
    assert_eq!(manager.next_request_id().raw(), REQUEST_ID_CEILING);

    let exhausted = std::panic::catch_unwind(|| manager.next_request_id());
    assert!(exhausted.is_err(), "allocator handed out an id past the ceiling");
}
