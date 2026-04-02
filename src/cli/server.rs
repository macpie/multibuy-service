use crate::{
    settings::Settings,
    state::State,
    tasks::{cleanup::CacheCleanup, grpc_server::GrpcServer},
};
use task_manager::TaskManager;

#[derive(Debug, clap::Args)]
pub struct Server {}

impl Server {
    pub async fn run(&self, settings: &Settings) -> anyhow::Result<()> {
        tracing::info!("starting server");

        let grpc_state = State::new(settings)?;
        let cache_cleanup = CacheCleanup::new(&grpc_state, settings.cleanup_timeout);
        let grpc_listen = settings.grpc_listen;

        TaskManager::builder()
            .add_named("grpc", GrpcServer::new(grpc_state, grpc_listen))
            .add_named("cache-cleanup", cache_cleanup)
            .build()
            .start()
            .await
            .map_err(anyhow::Error::from)
    }
}
