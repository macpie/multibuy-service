use anyhow::Result;
use helium_proto::services::multi_buy::{multi_buy_client::MultiBuyClient, MultiBuyIncReqV1};
use helium_proto::Region;

#[derive(Debug, clap::Args)]
pub struct Client {
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

impl Client {
    pub async fn run(&self) -> Result<()> {
        let hotspot_key = match &self.hotspot_key {
            Some(key_b58) => bs58::decode(key_b58)
                .into_vec()
                .map_err(|e| anyhow::anyhow!("invalid base58 hotspot key: {}", e))?,
            None => vec![],
        };

        let region = match &self.region {
            Some(name) => Region::from_str_name(name)
                .ok_or_else(|| anyhow::anyhow!("unknown region: '{}'", name))?
                as i32,
            None => 0,
        };

        let channel = if self.url.starts_with("https") {
            let tls = tonic::transport::ClientTlsConfig::new().with_native_roots();
            tonic::transport::Channel::from_shared(self.url.clone())?
                .tls_config(tls)?
                .connect()
                .await?
        } else {
            tonic::transport::Channel::from_shared(self.url.clone())?
                .connect()
                .await?
        };

        let mut client = MultiBuyClient::new(channel);

        let req = MultiBuyIncReqV1 {
            key: self.key.clone(),
            hotspot_key,
            region,
        };

        let res = client.inc(req).await?.into_inner();

        println!("count: {}, denied: {}", res.count, res.denied);

        Ok(())
    }
}
