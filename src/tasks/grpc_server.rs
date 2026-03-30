use crate::state::State;
use helium_proto::services::multi_buy::Server as MultiBuyServer;
use std::net::SocketAddr;

pub struct GrpcServer {
    state: State,
    listen: SocketAddr,
}

impl GrpcServer {
    pub fn new(state: State, listen: SocketAddr) -> Self {
        Self { state, listen }
    }

    async fn run(self, shutdown: triggered::Listener) -> anyhow::Result<()> {
        tracing::info!("gRPC server listening @ {:?}", self.listen);

        tonic::transport::Server::builder()
            .add_service(MultiBuyServer::new(self.state))
            .serve_with_shutdown(self.listen, shutdown)
            .await?;

        tracing::info!("gRPC server stopped");
        Ok(())
    }
}

impl task_manager::ManagedTask for GrpcServer {
    fn start_task(self: Box<Self>, shutdown: triggered::Listener) -> task_manager::TaskFuture {
        task_manager::spawn(self.run(shutdown))
    }
}
