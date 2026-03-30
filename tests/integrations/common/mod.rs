use helium_proto::services::multi_buy::{
    multi_buy_client::MultiBuyClient, multi_buy_server::MultiBuyServer, MultiBuyIncReqV1,
    MultiBuyIncResV1,
};
use multi_buy_service::grpc::state::State;
use multi_buy_service::settings::Settings;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tonic::transport::Channel;

/// Find an available port by binding to port 0.
pub async fn available_port() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap()
}

/// Build a test Settings with the given overrides.
pub fn test_settings(denied_hotspots: Vec<String>, denied_regions: Vec<String>) -> Settings {
    Settings::new::<String>(None).map_or_else(
        |_| panic!("failed to create default settings"),
        |mut s| {
            s.denied_hotspots = denied_hotspots;
            s.denied_regions = denied_regions;
            s.grpc_listen = "127.0.0.1:0".parse().unwrap();
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

    // Give the server a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

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
