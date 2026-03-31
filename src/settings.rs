use config::{Config, Environment, File};
use humantime_serde::re::humantime;
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct Settings {
    /// Log level configuration (RUST_LOG compatible)
    #[serde(default = "default_log")]
    pub log: String,

    /// Custom tracing settings for dynamic log level updates
    #[serde(default)]
    pub custom_tracing: custom_tracing::Settings,
    /// Listen address for grpc requests. Default "0.0.0.0:6080"
    #[serde(default = "default_grpc_listen_addr")]
    pub grpc_listen: SocketAddr,
    /// Metrics settings
    #[serde(default)]
    pub metrics: crate::metrics::Settings,
    #[serde(default = "default_cleanup_timeout", with = "humantime_serde")]
    pub cleanup_timeout: Duration,
}

pub fn default_log() -> String {
    "INFO".to_string()
}

pub fn default_grpc_listen_addr() -> SocketAddr {
    "0.0.0.0:6080".parse().expect("invalid default socket addr")
}

pub fn default_cleanup_timeout() -> Duration {
    humantime::parse_duration("30 minutes").unwrap()
}

impl Settings {
    /// Load Settings from a given path. Settings are loaded from a given
    /// optional path and can be overriden with environment variables.
    ///
    /// Environemnt overrides have the same name as the entries in the settings
    /// file in uppercase and prefixed with "MB_". For example
    /// "MB_LOG" will override the log setting.
    pub fn new<P: AsRef<Path>>(path: Option<P>) -> Result<Self, config::ConfigError> {
        let mut builder = Config::builder();

        if let Some(file) = path {
            // Add optional settings file
            let filename = file.as_ref().to_str().expect("file name");
            builder = builder.add_source(File::with_name(filename).required(false));
        }
        // Add in settings from the environment (with a prefix of MB)
        // Eg.. `MB_LOG=DEBUG ./target/release/multi_buy_service` would set the `log` key
        builder
            .add_source(Environment::with_prefix("MB").prefix_separator("__"))
            .build()
            .and_then(|config| config.try_deserialize())
    }
}
