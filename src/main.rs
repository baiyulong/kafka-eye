use anyhow::Result;
use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber;

mod app;
mod config;
mod kafka;
mod ui;
mod utils;

use app::App;
use config::Config;

#[derive(Parser)]
#[command(name = "kafka-eye")]
#[command(about = "A terminal-based Kafka client with vim-like interface")]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "config.yaml")]
    config: String,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Kafka broker address
    #[arg(short, long)]
    broker: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.debug { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();

    info!("Starting Kafka Eye TUI client");

    // Load configuration
    let mut config = Config::load(&cli.config)?;
    
    // Override broker if provided via CLI
    if let Some(broker) = cli.broker {
        config.set_default_broker(broker);
    }

    // Create and run the application
    let mut app = App::new(config).await?;
    app.run().await?;

    info!("Kafka Eye client shutdown complete");
    Ok(())
}
