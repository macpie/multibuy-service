//! Example gRPC client for the MultiBuy service.
//!
//! Usage:
//!   cargo run --example client -- --url http://localhost:6080 --key <packet-key> [--hotspot-key <b58>] [--region <name>]

use anyhow::Result;
use helium_proto::services::multi_buy::{multi_buy_client::MultiBuyClient, MultiBuyIncReqV1};
use helium_proto::Region;

#[derive(Debug, clap::Parser)]
struct Args {
    /// Server URL (e.g., http://localhost:6080)
    #[clap(long)]
    url: String,
    /// Packet key to increment
    #[clap(long)]
    key: String,
    /// Base58-encoded hotspot public key
    #[clap(long)]
    hotspot_key: Option<String>,
    /// Region name (e.g., US915, EU868)
    #[clap(long)]
    region: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    use clap::Parser;
    let args = Args::parse();

    let hotspot_key = match &args.hotspot_key {
        Some(key_b58) => key_b58.as_bytes().to_vec(),
        None => vec![],
    };

    let region = match &args.region {
        Some(name) => Region::from_str_name(name)
            .ok_or_else(|| anyhow::anyhow!("unknown region: '{}'", name))?
            as i32,
        None => 0,
    };

    let channel = if args.url.starts_with("https") {
        let tls = tonic::transport::ClientTlsConfig::new().with_enabled_roots();
        tonic::transport::Channel::from_shared(args.url.clone())?
            .tls_config(tls)?
            .connect()
            .await?
    } else {
        tonic::transport::Channel::from_shared(args.url.clone())?
            .connect()
            .await?
    };

    let mut client = MultiBuyClient::new(channel);

    let req = MultiBuyIncReqV1 {
        key: args.key.clone(),
        hotspot_key,
        region,
    };

    let res = client.inc(req).await?.into_inner();

    println!("count: {}, denied: {}", res.count, res.denied);

    Ok(())
}
