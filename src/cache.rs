use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

#[derive(Debug, Clone)]
pub(crate) struct CacheValue {
    count: u32,
    pub(crate) created_at: Instant,
}

pub struct Cache {
    map: DashMap<String, CacheValue>,
    pub size: AtomicUsize,
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

impl Cache {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
            size: AtomicUsize::new(0),
        }
    }

    pub fn len(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn inc(&self, key: String) -> u32 {
        match self.map.entry(key) {
            Entry::Occupied(mut entry) => {
                let val = entry.get_mut();
                val.count += 1;
                val.count
            }
            Entry::Vacant(entry) => {
                entry.insert(CacheValue {
                    count: 1,
                    created_at: Instant::now(),
                });
                self.size.fetch_add(1, Ordering::Relaxed);
                crate::metrics::set_cache_size(self.len() as f64);
                1
            }
        }
    }

    pub fn remove_expired(&self, max_age: std::time::Duration) -> usize {
        let before = self.map.len();
        self.map.retain(|_, v| v.created_at.elapsed() < max_age);
        let after = self.map.len();
        let removed = before - after;
        self.size.store(after, Ordering::Relaxed);
        removed
    }
}
