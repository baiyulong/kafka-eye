pub mod client;
pub mod admin;
pub mod consumer;
pub mod producer;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

use crate::config::Config;

#[derive(Debug, Clone)]
pub enum KafkaEvent {
    Connected,
    Disconnected,
    MessageReceived(KafkaMessage),
    MessageSent(String), // topic name
    TopicsUpdated(Vec<String>),
    ConsumerGroupsUpdated(Vec<String>),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct KafkaMessage {
    pub topic: String,
    pub partition: i32,
    pub offset: i64,
    pub key: Option<String>,
    pub value: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub headers: HashMap<String, String>,
}

pub struct KafkaManager {
    client: Option<client::KafkaClient>,
    config: Config,
}

impl KafkaManager {
    pub async fn new(config: &Config) -> Result<Self> {
        // Try to create client if there's an active cluster, but don't fail if there isn't
        let client = if let Some((_, kafka_config)) = config.get_active_cluster() {
            match client::KafkaClient::new_from_config(kafka_config).await {
                Ok(client) => Some(client),
                Err(e) => {
                    warn!("Failed to initialize Kafka client: {}", e);
                    None
                }
            }
        } else {
            None
        };
        
        Ok(Self {
            client,
            config: config.clone(),
        })
    }

    pub async fn connect(&mut self, config: &crate::config::KafkaConfig) -> Result<()> {
        let mut client = client::KafkaClient::new_from_config(config).await?;
        client.connect().await?;
        self.client = Some(client);
        info!("Successfully connected to Kafka cluster");
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(ref mut client) = self.client {
            info!("Disconnecting from Kafka cluster...");
            client.disconnect().await?;
            info!("Disconnected from Kafka cluster");
        }
        self.client = None;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.client.as_ref().map_or(false, |client| client.is_connected())
    }

    pub async fn list_topics(&self) -> Result<Vec<String>> {
        match &self.client {
            Some(client) => client.list_topics().await,
            None => Ok(vec![]), // Return empty list when not connected
        }
    }

    pub async fn get_topic_metadata(&self, topic: &str) -> Result<TopicMetadata> {
        match &self.client {
            Some(client) => client.get_topic_metadata(topic).await,
            None => Err(anyhow::anyhow!("Not connected to Kafka cluster")),
        }
    }

    pub async fn create_topic(&self, topic: &str, partitions: u32, replication_factor: u16) -> Result<()> {
        match &self.client {
            Some(client) => client.create_topic(topic, partitions, replication_factor).await,
            None => Err(anyhow::anyhow!("Not connected to Kafka cluster")),
        }
    }

    pub async fn delete_topic(&self, topic: &str) -> Result<()> {
        match &self.client {
            Some(client) => client.delete_topic(topic).await,
            None => Err(anyhow::anyhow!("Not connected to Kafka cluster")),
        }
    }

    pub async fn produce_message(&self, topic: &str, key: Option<&str>, value: &str) -> Result<()> {
        match &self.client {
            Some(client) => client.produce_message(topic, key, value).await,
            None => Err(anyhow::anyhow!("Not connected to Kafka cluster")),
        }
    }

    pub async fn start_consuming(&mut self, topic: &str, group_id: &str) -> Result<()> {
        match &mut self.client {
            Some(client) => client.start_consuming(topic, group_id).await,
            None => Err(anyhow::anyhow!("Not connected to Kafka cluster")),
        }
    }

    pub async fn stop_consuming(&mut self) -> Result<()> {
        match &mut self.client {
            Some(client) => client.stop_consuming().await,
            None => Err(anyhow::anyhow!("Not connected to Kafka cluster")),
        }
    }

    pub async fn list_consumer_groups(&self) -> Result<Vec<String>> {
        match &self.client {
            Some(client) => client.list_consumer_groups().await,
            None => Ok(vec![]), // Return empty list when not connected
        }
    }
}

#[derive(Debug, Clone)]
pub struct TopicMetadata {
    pub name: String,
    pub partitions: Vec<PartitionMetadata>,
    pub configs: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct PartitionMetadata {
    pub id: i32,
    pub leader: Option<i32>,
    pub replicas: Vec<i32>,
    pub in_sync_replicas: Vec<i32>,
}
