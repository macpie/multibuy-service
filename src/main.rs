use anyhow::Result;
use clap::Parser;
use multi_buy_service::cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.run().await
}
