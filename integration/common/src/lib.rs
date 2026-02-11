use std::collections::BTreeSet;
use std::sync::Mutex;

static POOL: Mutex<Option<BTreeSet<i32>>> = Mutex::new(None);

const ID_RANGE: std::ops::Range<i32> = 200..400;

/// RAII guard for a unique client ID. Returns the ID to the pool on drop.
pub struct ClientId(i32);

impl ClientId {
    /// Acquire the lowest available client ID from the pool.
    ///
    /// # Panics
    /// Panics if all IDs in the pool are currently in use.
    pub fn get() -> Self {
        let mut pool = POOL.lock().unwrap();
        let set = pool.get_or_insert_with(|| ID_RANGE.collect());
        let id = *set.iter().next().expect("client ID pool exhausted");
        set.remove(&id);
        ClientId(id)
    }

    pub fn id(&self) -> i32 {
        self.0
    }
}

impl Drop for ClientId {
    fn drop(&mut self) {
        if let Ok(mut pool) = POOL.lock() {
            if let Some(set) = pool.as_mut() {
                set.insert(self.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assigns_unique_ids() {
        let a = ClientId::get();
        let b = ClientId::get();
        assert_ne!(a.id(), b.id());
    }

    #[test]
    fn returns_id_on_drop() {
        let id = {
            let c = ClientId::get();
            c.id()
        };
        // After drop, same ID should be available again
        let d = ClientId::get();
        assert!(d.id() <= id);
    }

    #[test]
    fn ids_in_expected_range() {
        let c = ClientId::get();
        assert!(ID_RANGE.contains(&c.id()));
    }
}
