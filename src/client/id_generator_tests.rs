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
fn test_id_generator_set() {
    let gen = IdGenerator::new(100);
    assert_eq!(gen.next(), 100);
    gen.set(200);
    assert_eq!(gen.next(), 200);
    assert_eq!(gen.next(), 201);
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

    // Test order ID update
    manager.set_order_id(100);
    assert_eq!(manager.next_order_id(), 100);
    assert_eq!(manager.next_order_id(), 101);
}
