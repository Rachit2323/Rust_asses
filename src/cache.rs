use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use uuid::Uuid;

/// Simple in-memory, per-user TTL cache.
///
/// NOTE: this is documented as the "in-memory cache" option allowed by the
/// assignment in place of Redis. It is process-local and not shared across
/// multiple server instances - for a production deployment this should be
/// swapped for Redis (the public API here is intentionally small so that
/// swap is a drop-in change).
pub struct TasksCache {
    ttl: Duration,
    entries: Mutex<HashMap<Uuid, (String, Instant)>>,
}

impl TasksCache {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            ttl: Duration::from_secs(ttl_seconds),
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Returns the cached JSON body for a user, if present and not expired.
    pub fn get(&self, user_id: &Uuid) -> Option<String> {
        let mut entries = self.entries.lock().unwrap();
        match entries.get(user_id) {
            Some((value, inserted_at)) if inserted_at.elapsed() < self.ttl => {
                Some(value.clone())
            }
            Some(_) => {
                entries.remove(user_id);
                None
            }
            None => None,
        }
    }

    pub fn set(&self, user_id: Uuid, value: String) {
        let mut entries = self.entries.lock().unwrap();
        entries.insert(user_id, (value, Instant::now()));
    }

    /// Invalidates the cached view for a single user. Called whenever a
    /// task assigned to that user is created, assigned, or updated.
    pub fn invalidate(&self, user_id: &Uuid) {
        let mut entries = self.entries.lock().unwrap();
        entries.remove(user_id);
    }
}
