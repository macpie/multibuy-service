use metrics_exporter_prometheus::PrometheusBuilder;
pub use settings::Settings;
use std::net::SocketAddr;

pub mod settings;

const HIT_TOTAL: &str = "multi_buy_hit_total";
const CACHE_SIZE: &str = "multi_buy_cache_size";
const CACHE_CLEANED_TOTAL: &str = "multi_buy_cache_cleaned_total";
const REQUEST_DURATION: &str = "multi_buy_request_duration_ms";

pub fn start_metrics(settings: &Settings) -> anyhow::Result<()> {
    install(settings.endpoint)
}

fn install(socket_addr: SocketAddr) -> anyhow::Result<()> {
    PrometheusBuilder::new()
        .with_http_listener(socket_addr)
        .install()
        .map_err(|e| anyhow::anyhow!("failed to install Prometheus scrape endpoint: {e}"))?;

    tracing::info!("Metrics scrape endpoint listening on {socket_addr}");

    Ok(())
}

pub fn increment_hit() {
    metrics::counter!(HIT_TOTAL).increment(1);
}

pub fn set_cache_size(size: f64) {
    metrics::gauge!(CACHE_SIZE).set(size);
}

pub fn increment_cache_cleaned(count: u64) {
    metrics::counter!(CACHE_CLEANED_TOTAL).increment(count);
}

pub fn record_request_duration(duration: std::time::Duration) {
    metrics::histogram!(REQUEST_DURATION).record(duration.as_millis() as f64);
}
