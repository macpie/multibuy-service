use helium_proto::services::multi_buy::{
    multi_buy_client::MultiBuyClient, multi_buy_server::MultiBuyServer, MultiBuyIncReqV1,
    MultiBuyIncResV1,
};
use multi_buy_service::settings::Settings;
use multi_buy_service::state::State;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tonic::transport::Channel;

/// Find an available port by binding to port 0.
pub async fn available_port() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap()
}

/// Build a test Settings with defaults.
pub fn test_settings() -> Settings {
    test_settings_with_cleanup(Duration::from_secs(60 * 30))
}

/// Build a test Settings with custom cleanup timeout.
pub fn test_settings_with_cleanup(cleanup_timeout: Duration) -> Settings {
    Settings::new::<String>(None).map_or_else(
        |_| panic!("failed to create default settings"),
        |mut s| {
            s.grpc_listen = "127.0.0.1:0".parse().unwrap();
            s.cleanup_timeout = cleanup_timeout;
            s
        },
    )
}

/// Start the gRPC server on the given address and return a shutdown trigger.
/// The server runs in a background task.
pub async fn start_server(settings: &Settings, addr: SocketAddr) -> triggered::Trigger {
    let state = State::new(settings).unwrap();
    let (trigger, shutdown) = triggered::trigger();

    let incoming = TcpListener::bind(addr).await.unwrap();
    let incoming_stream = tokio_stream::wrappers::TcpListenerStream::new(incoming);

    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(MultiBuyServer::new(state))
            .serve_with_incoming_shutdown(incoming_stream, shutdown)
            .await
            .unwrap();
    });

    trigger
}

/// Start the gRPC server and also run the cache cleanup task.
/// Returns (shutdown_trigger, cache_arc) for inspection.
pub async fn start_server_with_cleanup(
    settings: &Settings,
    addr: SocketAddr,
) -> triggered::Trigger {
    let state = State::new(settings).unwrap();
    let cache = state.cache();
    let cleanup_timeout = settings.cleanup_timeout;
    let (trigger, shutdown) = triggered::trigger();

    let incoming = TcpListener::bind(addr).await.unwrap();
    let incoming_stream = tokio_stream::wrappers::TcpListenerStream::new(incoming);

    let shutdown_clone = shutdown.clone();
    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(MultiBuyServer::new(state))
            .serve_with_incoming_shutdown(incoming_stream, shutdown_clone)
            .await
            .unwrap();
    });

    // Spawn cleanup task
    let shutdown_clone = shutdown;
    tokio::spawn(async move {
        use multi_buy_service::tasks::cleanup::CacheCleanup;
        let cleanup = CacheCleanup::from_cache(cache, cleanup_timeout);
        cleanup.run_until(shutdown_clone).await.unwrap();
    });

    trigger
}

/// Connect a MultiBuyClient to the given address.
pub async fn connect_client(addr: SocketAddr) -> MultiBuyClient<Channel> {
    let url = format!("http://{addr}");
    MultiBuyClient::connect(url).await.unwrap()
}

/// Send an inc request with the given key, hotspot_key, and region.
pub async fn inc(
    client: &mut MultiBuyClient<Channel>,
    key: &str,
    hotspot_key: Vec<u8>,
    region: i32,
) -> MultiBuyIncResV1 {
    let req = MultiBuyIncReqV1 {
        key: key.to_string(),
        hotspot_key,
        region,
    };
    client.inc(req).await.unwrap().into_inner()
}
