use metrics_exporter_prometheus::PrometheusBuilder;
pub use settings::Settings;
use std::net::SocketAddr;

pub mod settings;

const HIT_TOTAL: &str = "multi_buy_hit_total";
const DENIED_TOTAL: &str = "multi_buy_denied_total";
const CACHE_SIZE: &str = "multi_buy_cache_size";
const CACHE_CLEANED_TOTAL: &str = "multi_buy_cache_cleaned_total";

pub fn start_metrics(settings: &Settings) -> anyhow::Result<()> {
    install(settings.endpoint)
}

fn install(socket_addr: SocketAddr) -> anyhow::Result<()> {
    if let Err(e) = PrometheusBuilder::new()
        .with_http_listener(socket_addr)
        .install()
    {
        tracing::error!("Failed to install Prometheus scrape endpoint: {e}");
    } else {
        tracing::info!("Metrics scrape endpoint listening on {socket_addr}");
    }

    Ok(())
}

pub fn increment_hit() {
    metrics::counter!(HIT_TOTAL).increment(1);
}

pub fn increment_denied() {
    metrics::counter!(DENIED_TOTAL).increment(1);
}

pub fn set_cache_size(size: f64) {
    metrics::gauge!(CACHE_SIZE).set(size);
}

pub fn increment_cache_cleaned(count: u64) {
    metrics::counter!(CACHE_CLEANED_TOTAL).increment(count);
}
