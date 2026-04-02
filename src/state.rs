use crate::cache::Cache;
use crate::settings::Settings;
use helium_proto::services::multi_buy::{multi_buy_server, MultiBuyIncReqV1, MultiBuyIncResV1};
use std::sync::Arc;
use tonic::Request;

pub struct State {
    cache: Arc<Cache>,
}

impl State {
    pub fn new(_settings: &Settings) -> anyhow::Result<Self> {
        Ok(Self {
            cache: Arc::new(Cache::new()),
        })
    }

    pub fn cache(&self) -> Arc<Cache> {
        self.cache.clone()
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
        let new_count = self.cache.inc(multi_buy_req.key.clone());

        tracing::info!("Key={} Count={}", multi_buy_req.key, new_count);

        crate::metrics::record_request_duration(start.elapsed());

        Ok(tonic::Response::new(MultiBuyIncResV1 {
            count: new_count,
            denied: false,
        }))
    }
}
