use anyhow::Result;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::{Message, TopicPartitionList};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use super::{TopicMetadata, PartitionMetadata};

pub struct KafkaClient {
    admin_client: AdminClient<DefaultClientContext>,
    producer: FutureProducer,
    consumer: Option<StreamConsumer>,
    config: ClientConfig,
    connected: bool,
}

impl KafkaClient {
    pub async fn new(config: &Config) -> Result<Self> {
        let mut client_config = ClientConfig::new();
        
        // Basic configuration
        client_config.set("bootstrap.servers", &config.kafka.brokers.join(","));
        client_config.set("client.id", &config.kafka.client_id);
        
        // Security configuration
        if let Some(security_config) = &config.kafka.security {
            client_config.set("security.protocol", &security_config.protocol);
            
            if let Some(sasl_config) = &security_config.sasl {
                client_config.set("sasl.mechanism", &sasl_config.mechanism);
                if let Some(username) = &sasl_config.username {
                    client_config.set("sasl.username", username);
                }
                if let Some(password) = &sasl_config.password {
                    client_config.set("sasl.password", password);
                }
            }
            
            if let Some(ssl_config) = &security_config.ssl {
                if let Some(ca_location) = &ssl_config.ca_location {
                    client_config.set("ssl.ca.location", ca_location);
                }
                if let Some(cert_location) = &ssl_config.certificate_location {
                    client_config.set("ssl.certificate.location", cert_location);
                }
                if let Some(key_location) = &ssl_config.key_location {
                    client_config.set("ssl.key.location", key_location);
                }
            }
        }

        // Create clients
        let admin_client: AdminClient<DefaultClientContext> = client_config.create()?;
        let producer: FutureProducer = client_config.create()?;

        Ok(Self {
            admin_client,
            producer,
            consumer: None,
            config: client_config,
            connected: false,
        })
    }

    pub async fn connect(&mut self) -> Result<()> {
        // Test connection by listing topics
        let _topics = self.list_topics().await?;
        self.connected = true;
        info!("Successfully connected to Kafka cluster");
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(consumer) = &self.consumer {
            consumer.unsubscribe();
        }
        self.consumer = None;
        self.connected = false;
        info!("Disconnected from Kafka cluster");
        Ok(())
    }

    pub async fn list_topics(&self) -> Result<Vec<String>> {
        let timeout = Duration::from_secs(10);
        let metadata = self.admin_client.inner().fetch_metadata(None, timeout)?;
        
        let topics = metadata
            .topics()
            .iter()
            .filter(|topic| !topic.name().starts_with("__")) // Filter out internal topics
            .map(|topic| topic.name().to_string())
            .collect();

        debug!("Listed {} topics", topics.len());
        Ok(topics)
    }

    pub async fn get_topic_metadata(&self, topic_name: &str) -> Result<TopicMetadata> {
        let timeout = Duration::from_secs(10);
        let metadata = self.admin_client.inner().fetch_metadata(Some(topic_name), timeout)?;
        
        if let Some(topic) = metadata.topics().iter().find(|t| t.name() == topic_name) {
            let partitions = topic
                .partitions()
                .iter()
                .map(|p| PartitionMetadata {
                    id: p.id(),
                    leader: if p.leader() >= 0 { Some(p.leader()) } else { None },
                    replicas: p.replicas().to_vec(),
                    in_sync_replicas: p.isr().to_vec(),
                })
                .collect();

            Ok(TopicMetadata {
                name: topic_name.to_string(),
                partitions,
                configs: HashMap::new(), // TODO: Fetch topic configs
            })
        } else {
            Err(anyhow::anyhow!("Topic '{}' not found", topic_name))
        }
    }

    pub async fn create_topic(&self, topic_name: &str, partitions: u32, replication_factor: u16) -> Result<()> {
        let new_topic = NewTopic::new(
            topic_name,
            partitions as i32,
            TopicReplication::Fixed(replication_factor as i32),
        );

        let admin_opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(30)));
        let results = self.admin_client.create_topics(&[new_topic], &admin_opts).await?;

        for result in results {
            match result {
                Ok(topic) => {
                    info!("Successfully created topic: {}", topic);
                }
                Err((topic, error)) => {
                    error!("Failed to create topic {}: {}", topic, error);
                    return Err(anyhow::anyhow!("Failed to create topic {}: {}", topic, error));
                }
            }
        }

        Ok(())
    }

    pub async fn delete_topic(&self, topic_name: &str) -> Result<()> {
        let admin_opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(30)));
        let results = self.admin_client.delete_topics(&[topic_name], &admin_opts).await?;

        for result in results {
            match result {
                Ok(topic) => {
                    info!("Successfully deleted topic: {}", topic);
                }
                Err((topic, error)) => {
                    error!("Failed to delete topic {}: {}", topic, error);
                    return Err(anyhow::anyhow!("Failed to delete topic {}: {}", topic, error));
                }
            }
        }

        Ok(())
    }

    pub async fn produce_message(&self, topic: &str, key: Option<&str>, value: &str) -> Result<()> {
        let mut record = FutureRecord::to(topic).payload(value);
        
        if let Some(k) = key {
            record = record.key(k);
        }

        let delivery_status = self.producer.send(record, Duration::from_secs(10)).await;

        match delivery_status {
            Ok((partition, offset)) => {
                debug!("Message delivered to topic: {}, partition: {}, offset: {}", topic, partition, offset);
                Ok(())
            }
            Err((error, _)) => {
                error!("Failed to deliver message: {}", error);
                Err(anyhow::anyhow!("Failed to deliver message: {}", error))
            }
        }
    }

    pub async fn start_consuming(&mut self, topic: &str, group_id: &str) -> Result<()> {
        let mut consumer_config = self.config.clone();
        consumer_config.set("group.id", group_id);
        consumer_config.set("enable.auto.commit", "true");
        consumer_config.set("auto.offset.reset", "earliest");

        let consumer: StreamConsumer = consumer_config.create()?;
        consumer.subscribe(&[topic])?;
        
        self.consumer = Some(consumer);
        info!("Started consuming from topic: {} with group: {}", topic, group_id);
        Ok(())
    }

    pub async fn stop_consuming(&mut self) -> Result<()> {
        if let Some(consumer) = &self.consumer {
            consumer.unsubscribe();
        }
        self.consumer = None;
        info!("Stopped consuming");
        Ok(())
    }

    pub async fn list_consumer_groups(&self) -> Result<Vec<String>> {
        // Note: This is a simplified implementation
        // In a real implementation, you would use the admin client to fetch consumer groups
        // For now, we'll return an empty list as rdkafka doesn't have a direct method for this
        warn!("Consumer group listing not fully implemented yet");
        Ok(vec![])
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }
}
