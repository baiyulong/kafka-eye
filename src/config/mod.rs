use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub kafka: KafkaConfig,
    pub ui: UiConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    pub brokers: Vec<String>,
    pub client_id: String,
    pub security: Option<SecurityConfig>,
    pub producer: ProducerConfig,
    pub consumer: ConsumerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub protocol: String,
    pub sasl: Option<SaslConfig>,
    pub ssl: Option<SslConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaslConfig {
    pub mechanism: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslConfig {
    pub ca_location: Option<String>,
    pub certificate_location: Option<String>,
    pub key_location: Option<String>,
    pub key_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProducerConfig {
    pub acks: String,
    pub compression_type: String,
    pub batch_size: u32,
    pub linger_ms: u32,
}

impl Default for ProducerConfig {
    fn default() -> Self {
        Self {
            acks: "all".to_string(),
            compression_type: "none".to_string(),
            batch_size: 16384,
            linger_ms: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerConfig {
    pub auto_offset_reset: String,
    pub enable_auto_commit: bool,
    pub auto_commit_interval_ms: u32,
    pub session_timeout_ms: u32,
    pub heartbeat_interval_ms: u32,
}

impl Default for ConsumerConfig {
    fn default() -> Self {
        Self {
            auto_offset_reset: "earliest".to_string(),
            enable_auto_commit: true,
            auto_commit_interval_ms: 5000,
            session_timeout_ms: 30000,
            heartbeat_interval_ms: 3000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: String,
    pub refresh_interval_ms: u64,
    pub max_messages: usize,
    pub vim_mode: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            refresh_interval_ms: 1000,
            max_messages: 1000,
            vim_mode: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            kafka: KafkaConfig {
                brokers: vec!["localhost:9092".to_string()],
                client_id: "kafka-eye".to_string(),
                security: None,
                producer: ProducerConfig {
                    acks: "all".to_string(),
                    compression_type: "none".to_string(),
                    batch_size: 16384,
                    linger_ms: 0,
                },
                consumer: ConsumerConfig {
                    auto_offset_reset: "earliest".to_string(),
                    enable_auto_commit: true,
                    auto_commit_interval_ms: 5000,
                    session_timeout_ms: 30000,
                    heartbeat_interval_ms: 3000,
                },
            },
            ui: UiConfig {
                theme: "default".to_string(),
                refresh_interval_ms: 1000,
                max_messages: 1000,
                vim_mode: true,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file: None,
            },
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        if !path.exists() {
            warn!("Configuration file not found at {:?}, creating default config", path);
            let default_config = Config::default();
            default_config.save(path)?;
            return Ok(default_config);
        }

        let content = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        
        info!("Loaded configuration from {:?}", path);
        Ok(config)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_yaml::to_string(self)?;
        fs::write(path, content)?;
        
        info!("Saved configuration to {:?}", path);
        Ok(())
    }

    pub fn set_default_broker(&mut self, broker: String) {
        self.kafka.brokers = vec![broker];
    }

    pub fn add_broker(&mut self, broker: String) {
        if !self.kafka.brokers.contains(&broker) {
            self.kafka.brokers.push(broker);
        }
    }

    pub fn remove_broker(&mut self, broker: &str) {
        self.kafka.brokers.retain(|b| b != broker);
    }

    pub fn set_security_config(&mut self, security: SecurityConfig) {
        self.kafka.security = Some(security);
    }

    pub fn clear_security_config(&mut self) {
        self.kafka.security = None;
    }

    pub fn validate(&self) -> Result<()> {
        if self.kafka.brokers.is_empty() {
            return Err(anyhow::anyhow!("At least one Kafka broker must be configured"));
        }

        for broker in &self.kafka.brokers {
            if !broker.contains(':') {
                return Err(anyhow::anyhow!("Invalid broker format: {}. Expected format: host:port", broker));
            }
        }

        if self.kafka.client_id.is_empty() {
            return Err(anyhow::anyhow!("Client ID cannot be empty"));
        }

        // Validate security configuration if present
        if let Some(security) = &self.kafka.security {
            match security.protocol.as_str() {
                "PLAINTEXT" | "SSL" | "SASL_PLAINTEXT" | "SASL_SSL" => {}
                _ => return Err(anyhow::anyhow!("Invalid security protocol: {}", security.protocol)),
            }

            if security.protocol.contains("SASL") && security.sasl.is_none() {
                return Err(anyhow::anyhow!("SASL configuration is required when using SASL protocol"));
            }

            if security.protocol.contains("SSL") && security.ssl.is_none() {
                return Err(anyhow::anyhow!("SSL configuration is required when using SSL protocol"));
            }
        }

        Ok(())
    }
}

// Example configuration template for different environments
impl Config {
    pub fn development() -> Self {
        Self {
            kafka: KafkaConfig {
                brokers: vec!["localhost:9092".to_string()],
                client_id: "kafka-eye-dev".to_string(),
                security: None,
                producer: ProducerConfig::default(),
                consumer: ConsumerConfig::default(),
            },
            logging: LoggingConfig {
                level: "debug".to_string(),
                file: Some("kafka-eye-dev.log".to_string()),
            },
            ..Default::default()
        }
    }

    pub fn production() -> Self {
        Self {
            kafka: KafkaConfig {
                brokers: vec!["broker1:9092".to_string(), "broker2:9092".to_string(), "broker3:9092".to_string()],
                client_id: "kafka-eye-prod".to_string(),
                security: Some(SecurityConfig {
                    protocol: "SASL_SSL".to_string(),
                    sasl: Some(SaslConfig {
                        mechanism: "SCRAM-SHA-256".to_string(),
                        username: Some("kafka-user".to_string()),
                        password: None, // Should be set via environment variable
                    }),
                    ssl: Some(SslConfig {
                        ca_location: Some("/etc/ssl/certs/ca-certificates.crt".to_string()),
                        certificate_location: None,
                        key_location: None,
                        key_password: None,
                    }),
                }),
                producer: ProducerConfig::default(),
                consumer: ConsumerConfig::default(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file: Some("/var/log/kafka-eye.log".to_string()),
            },
            ..Default::default()
        }
    }
}
