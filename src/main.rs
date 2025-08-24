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

    // Create default configuration
    let mut config = Config {
        clusters: {
            let mut clusters = std::collections::HashMap::new();
            // Add a default local cluster
            clusters.insert(
                "local".to_string(),
                config::KafkaConfig {
                    brokers: vec!["localhost:9092".to_string()],
                    client_id: "kafka-eye".to_string(),
                    security: None,
                    producer: config::ProducerConfig::default(),
                    consumer: config::ConsumerConfig::default(),
                },
            );
            clusters
        },
        active_cluster: Some("local".to_string()),
        ui: config::UiConfig {
            theme: config::Theme::default(),
            refresh_interval_ms: 1000,
            max_messages: 1000,
            vim_mode: true,
        },
        logging: config::LoggingConfig {
            level: "info".to_string(),
            file: None,
        },
    };

    // Override broker if provided via CLI
    if let Some(broker) = cli.broker {
        let _ = config.set_default_broker(broker);
    }

    // Create and run the application
    let mut app = App::new(config).await?;
    app.run().await?;

    info!("Kafka Eye client shutdown complete");
    Ok(())
}
