use crate::settings::Settings;
use helium_proto::services::multi_buy::{multi_buy_server, MultiBuyIncReqV1, MultiBuyIncResV1};
use helium_proto::Region;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tonic::Request;
use tracing::{info, warn};

#[derive(Debug, Copy, Clone)]
pub(crate) struct CacheValue {
    count: u32,
    pub(crate) timestamp: u128,
}

pub struct State {
    cache: Arc<Mutex<HashMap<String, CacheValue>>>,
    denied_hotspots: HashSet<Vec<u8>>,
    denied_regions: HashSet<i32>,
}

impl State {
    pub fn new(settings: &Settings) -> anyhow::Result<Self> {
        let denied_hotspots: HashSet<Vec<u8>> = settings
            .denied_hotspots
            .iter()
            .filter_map(|key_b58| {
                bs58::decode(key_b58).into_vec().ok().or_else(|| {
                    warn!("Invalid base58 hotspot key in deny list: {}", key_b58);
                    None
                })
            })
            .collect();

        let denied_regions: HashSet<i32> = settings
            .denied_regions
            .iter()
            .filter_map(|name| {
                Region::from_str_name(name).map(|r| r as i32).or_else(|| {
                    warn!("Unknown region in deny list: {}", name);
                    None
                })
            })
            .collect();

        if !denied_hotspots.is_empty() {
            info!("Denying {} hotspot(s)", denied_hotspots.len());
        }
        if !denied_regions.is_empty() {
            info!("Denying {} region(s)", denied_regions.len());
        }

        Ok(Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            denied_hotspots,
            denied_regions,
        })
    }

    pub(crate) fn cache(&self) -> Arc<Mutex<HashMap<String, CacheValue>>> {
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
        crate::metrics::increment_hit();

        let multi_buy_req = request.into_inner();
        let denied = self.is_denied(&multi_buy_req);

        if denied {
            crate::metrics::increment_denied();
        }

        let key = multi_buy_req.key;
        let mut cache = self.cache.lock().await;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis();

        let cached_value: CacheValue = match cache.get(&key) {
            None => {
                crate::metrics::set_cache_size(cache.len() as f64 + 1.0);
                CacheValue {
                    count: 0,
                    timestamp: now,
                }
            }
            Some(&cached_value) => cached_value,
        };

        let new_count = cached_value.count + 1;

        cache.insert(
            key.clone(),
            CacheValue {
                count: new_count,
                timestamp: cached_value.timestamp,
            },
        );

        info!("Key={} Count={} Denied={}", key, new_count, denied);

        Ok(tonic::Response::new(MultiBuyIncResV1 {
            count: new_count,
            denied,
        }))
    }
}
