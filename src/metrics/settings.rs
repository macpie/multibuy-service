use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Settings {
    /// Scrape endpoint for metrics
    #[serde(default = "default_metrics_endpoint")]
    pub endpoint: SocketAddr,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            endpoint: default_metrics_endpoint(),
        }
    }
}

fn default_metrics_endpoint() -> SocketAddr {
    "0.0.0.0:19011".parse().unwrap()
}
