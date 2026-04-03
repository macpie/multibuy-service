use crate::cache::Cache;
use crate::deny_lists::DenyLists;
use crate::settings::Settings;
use helium_proto::services::multi_buy::{multi_buy_server, MultiBuyIncReqV1, MultiBuyIncResV1};
use std::sync::Arc;
use tonic::Request;

pub struct State {
    cache: Arc<Cache>,
    deny_lists: DenyLists,
}

impl State {
    pub fn new(settings: &Settings) -> anyhow::Result<Self> {
        let deny_lists =
            DenyLists::from_config(&settings.denied_hotspots, &settings.denied_regions)?;
        Ok(Self {
            cache: Arc::new(Cache::new()),
            deny_lists,
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
        let denied = self.deny_lists.is_denied(&multi_buy_req);
        let count = self.cache.inc(multi_buy_req.key.clone());
        let hotspot = bs58::encode(&multi_buy_req.hotspot_key).into_string();

        if denied {
            tracing::info!(
                key = %multi_buy_req.key,
                count,
                hotspot,
                region = %multi_buy_req.region,
                "denied by deny list"
            );
            crate::metrics::increment_denied();
        } else {
            tracing::debug!(
                key = %multi_buy_req.key,
                count,
                hotspot,
                region = %multi_buy_req.region,
                "got inc req"
            );
        }

        crate::metrics::record_request_duration(start.elapsed());

        Ok(tonic::Response::new(MultiBuyIncResV1 { count, denied }))
    }
}
