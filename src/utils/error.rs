use thiserror::Error;

#[derive(Error, Debug)]
pub enum KafkaEyeError {
    #[error("Kafka error: {0}")]
    Kafka(#[from] rdkafka::error::KafkaError),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("UI error: {0}")]
    Ui(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_yaml::Error),
    
    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, KafkaEyeError>;
