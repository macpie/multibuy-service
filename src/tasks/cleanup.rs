use crate::cache::Cache;
use crate::state::State;
use std::sync::Arc;
use std::time::Duration;

pub struct CacheCleanup {
    cache: Arc<Cache>,
    cleanup_timeout: Duration,
}

impl CacheCleanup {
    pub fn new(state: &State, cleanup_timeout: Duration) -> Self {
        Self {
            cache: state.cache(),
            cleanup_timeout,
        }
    }

    pub fn from_cache(cache: Arc<Cache>, cleanup_timeout: Duration) -> Self {
        Self {
            cache,
            cleanup_timeout,
        }
    }

    pub async fn run_until(self, shutdown: triggered::Listener) -> anyhow::Result<()> {
        self.run(shutdown).await
    }

    async fn run(self, shutdown: triggered::Listener) -> anyhow::Result<()> {
        let mut interval = tokio::time::interval(self.cleanup_timeout);

        loop {
            tokio::select! {
                biased;
                _ = shutdown.clone() => {
                    tracing::info!("shutdown signal received, stopping cache cleanup");
                    break;
                },
                _ = interval.tick() => {
                    let removed = self.cache.remove_expired(self.cleanup_timeout);
                    tracing::info!("cleaned {} entries", removed);
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
