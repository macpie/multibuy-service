use crate::grpc::state::{CacheValue, State};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

pub struct CacheCleanup {
    cache: Arc<Mutex<HashMap<String, CacheValue>>>,
    cleanup_timeout: Duration,
}

impl CacheCleanup {
    pub fn new(state: &State, cleanup_timeout: Duration) -> Self {
        Self {
            cache: state.cache(),
            cleanup_timeout,
        }
    }

    async fn run(self, shutdown: triggered::Listener) -> anyhow::Result<()> {
        let batch_timer = tokio::time::sleep(self.cleanup_timeout);
        tokio::pin!(batch_timer);

        loop {
            tokio::select! {
                biased;
                _ = shutdown.clone() => {
                    tracing::info!("shutdown signal received, stopping cache cleanup");
                    break;
                },
                _ = &mut batch_timer => {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_millis();

                    let mut cache = self.cache.lock().await;
                    let size_before = cache.len() as f64;

                    cache.retain(|_, v| v.timestamp > now - self.cleanup_timeout.as_millis());

                    let size_after = cache.len() as f64;
                    let cleaned = (size_before - size_after) as u64;
                    crate::metrics::set_cache_size(size_after);
                    crate::metrics::increment_cache_cleaned(cleaned);
                    tracing::info!("cleaned {} entries", cleaned);

                    batch_timer.as_mut().reset(tokio::time::Instant::now() + self.cleanup_timeout);
                }
            }
        }

        Ok(())
    }
}

impl task_manager::ManagedTask for CacheCleanup {
    fn start_task(self: Box<Self>, shutdown: triggered::Listener) -> task_manager::TaskFuture {
        task_manager::spawn(self.run(shutdown))
    }
}
