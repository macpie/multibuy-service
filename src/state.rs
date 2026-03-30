use crate::cache::Cache;
use crate::settings::Settings;
use helium_proto::services::multi_buy::{multi_buy_server, MultiBuyIncReqV1, MultiBuyIncResV1};
use helium_proto::Region;
use std::collections::HashSet;
use std::sync::Arc;
use tonic::Request;

pub struct State {
    cache: Arc<Cache>,
    denied_hotspots: HashSet<Vec<u8>>,
    denied_regions: HashSet<i32>,
}

impl State {
    pub fn new(settings: &Settings) -> anyhow::Result<Self> {
        let mut denied_hotspots = HashSet::new();
        for key_b58 in &settings.denied_hotspots {
            let decoded = bs58::decode(key_b58)
                .into_vec()
                .map_err(|e| anyhow::anyhow!("invalid base58 hotspot key '{}': {}", key_b58, e))?;
            denied_hotspots.insert(decoded);
        }

        let mut denied_regions = HashSet::new();
        for name in &settings.denied_regions {
            let region = Region::from_str_name(name)
                .ok_or_else(|| anyhow::anyhow!("unknown region in deny list: '{}'", name))?;
            denied_regions.insert(region as i32);
        }

        if !denied_hotspots.is_empty() {
            tracing::info!("Denying {} hotspot(s)", denied_hotspots.len());
        }
        if !denied_regions.is_empty() {
            tracing::info!("Denying {} region(s)", denied_regions.len());
        }

        Ok(Self {
            cache: Arc::new(Cache::new()),
            denied_hotspots,
            denied_regions,
        })
    }

    pub fn cache(&self) -> Arc<Cache> {
        self.cache.clone()
    }

    fn is_denied(&self, req: &MultiBuyIncReqV1) -> bool {
        if !req.hotspot_key.is_empty() && self.denied_hotspots.contains(&req.hotspot_key[..]) {
            return true;
        }
        if req.region != 0 && self.denied_regions.contains(&req.region) {
            return true;
        }
        false
    }
}

#[tonic::async_trait]
impl multi_buy_server::MultiBuy for State {
    async fn inc(
        &self,
        request: Request<MultiBuyIncReqV1>,
    ) -> Result<tonic::Response<MultiBuyIncResV1>, tonic::Status> {
        let start = std::time::Instant::now();
        crate::metrics::increment_hit();

        let multi_buy_req = request.into_inner();
        let denied = self.is_denied(&multi_buy_req);

        if denied {
            crate::metrics::increment_denied();
        }

        let new_count = self.cache.inc(multi_buy_req.key.clone());

        tracing::info!(
            "Key={} Count={} Denied={}",
            multi_buy_req.key,
            new_count,
            denied
        );

        crate::metrics::record_request_duration(start.elapsed());

        Ok(tonic::Response::new(MultiBuyIncResV1 {
            count: new_count,
            denied,
        }))
    }
}
