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
    assert_eq!(gen.current(), INITIAL_REQUEST_ID);
    assert_eq!(gen.next(), INITIAL_REQUEST_ID);
    assert_eq!(gen.next(), INITIAL_REQUEST_ID + 1);
}

#[test]
fn test_client_id_manager() {
    let manager = ClientIdManager::new(50);

    // Test request IDs
    assert_eq!(manager.current_request_id(), INITIAL_REQUEST_ID);
    assert_eq!(manager.next_request_id(), INITIAL_REQUEST_ID);
    assert_eq!(manager.next_request_id(), INITIAL_REQUEST_ID + 1);

    // Test order IDs
    assert_eq!(manager.current_order_id(), 50);
    assert_eq!(manager.next_order_id(), 50);
    assert_eq!(manager.next_order_id(), 51);

    // Test order ID raise
    manager.raise_order_id(100);
    assert_eq!(manager.next_order_id(), 100);
    assert_eq!(manager.next_order_id(), 101);

    // Raising below the allocated high-water mark must not reissue IDs.
    manager.raise_order_id(100);
    assert_eq!(manager.next_order_id(), 102);
}
