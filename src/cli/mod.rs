pub mod server;

use crate::settings::Settings;
use anyhow::Result;
use clap::Parser;
use server::Server;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    /// Optional configuration file to use. If present, the toml file at the
    /// given path will be loaded. Environment variables can override the
    /// settings in the given file.
    #[clap(short = 'c')]
    config: Option<PathBuf>,
    #[clap(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Run as a long-running service that continuously executes compaction jobs
    Server(Server),
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        let settings = Settings::new(self.config)?;

        // Initialize custom tracing
        custom_tracing::init(settings.log.clone(), settings.custom_tracing.clone()).await?;

        crate::metrics::start_metrics(&settings.metrics)?;

        match self.cmd {
            Cmd::Server(server) => server.run(&settings).await,
        }
    }
}
