pub mod client;
pub mod admin;
pub mod consumer;
pub mod producer;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info};

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
    client: client::KafkaClient,
    config: Config,
}

impl KafkaManager {
    pub async fn new(config: Config) -> Result<Self> {
        let client = client::KafkaClient::new(&config).await?;
        
        Ok(Self {
            client,
            config,
        })
    }

    pub async fn connect(&mut self) -> Result<()> {
        info!("Connecting to Kafka cluster...");
        self.client.connect().await?;
        info!("Successfully connected to Kafka cluster");
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Kafka cluster...");
        self.client.disconnect().await?;
        info!("Disconnected from Kafka cluster");
        Ok(())
    }

    pub async fn list_topics(&self) -> Result<Vec<String>> {
        self.client.list_topics().await
    }

    pub async fn get_topic_metadata(&self, topic: &str) -> Result<TopicMetadata> {
        self.client.get_topic_metadata(topic).await
    }

    pub async fn create_topic(&self, topic: &str, partitions: u32, replication_factor: u16) -> Result<()> {
        self.client.create_topic(topic, partitions, replication_factor).await
    }

    pub async fn delete_topic(&self, topic: &str) -> Result<()> {
        self.client.delete_topic(topic).await
    }

    pub async fn produce_message(&self, topic: &str, key: Option<&str>, value: &str) -> Result<()> {
        self.client.produce_message(topic, key, value).await
    }

    pub async fn start_consuming(&self, topic: &str, group_id: &str) -> Result<()> {
        self.client.start_consuming(topic, group_id).await
    }

    pub async fn stop_consuming(&self) -> Result<()> {
        self.client.stop_consuming().await
    }

    pub async fn list_consumer_groups(&self) -> Result<Vec<String>> {
        self.client.list_consumer_groups().await
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
