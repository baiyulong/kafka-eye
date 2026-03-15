#![allow(dead_code)]

use crate::app::*;
use crate::config::{AuthConfig, ClusterConfig};
use anyhow::Result;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::metadata::Metadata;
use rdkafka::message::Headers;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::TopicPartitionList;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;

/// Commands sent from UI to Kafka backend
#[derive(Debug)]
pub enum KafkaCommand {
    Connect(ClusterConfig),
    Disconnect,
    FetchMetadata,
    FetchTopics,
    FetchTopicDetail(String),
    CreateTopic {
        name: String,
        partitions: i32,
        replication_factor: i32,
        config: HashMap<String, String>,
    },
    DeleteTopic(String),
    StartConsuming {
        topic: String,
        offset_mode: OffsetMode,
    },
    StopConsuming,
    FetchConsumerGroups,
    FetchConsumerGroupDetail(String),
    ProduceMessage {
        topic: String,
        key: Option<String>,
        value: String,
        headers: Vec<(String, String)>,
    },
    TestConnection(ClusterConfig),
}

/// Responses from Kafka backend to UI
#[derive(Debug, Clone)]
pub enum KafkaResponse {
    Connected(String),
    Disconnected,
    ConnectionFailed(String),
    MetadataUpdate {
        controller_id: Option<i32>,
        broker_count: usize,
        brokers_online: Vec<i32>,
        topic_count: usize,
        partition_count: usize,
    },
    TopicList(Vec<TopicInfo>),
    TopicDetail {
        name: String,
        partitions: Vec<PartitionInfo>,
    },
    TopicCreated(String),
    TopicDeleted(String),
    Messages(Vec<KafkaMessage>),
    ConsumerGroupList(Vec<ConsumerGroupInfo>),
    ConsumerGroupDetail(ConsumerGroupInfo),
    MessageProduced {
        topic: String,
        partition: i32,
        offset: i64,
    },
    Error(String),
    ConnectionTestResult {
        cluster_name: String,
        success: bool,
        message: String,
    },
}

fn build_client_config(cluster: &ClusterConfig) -> ClientConfig {
    let mut config = ClientConfig::new();
    config.set("bootstrap.servers", &cluster.brokers);
    config.set("socket.timeout.ms", "10000");
    config.set("session.timeout.ms", "10000");

    match &cluster.auth {
        AuthConfig::None => {}
        AuthConfig::SaslPlain { username, password } => {
            config.set("security.protocol", "SASL_PLAINTEXT");
            config.set("sasl.mechanism", "PLAIN");
            config.set("sasl.username", username);
            config.set("sasl.password", password);
        }
        AuthConfig::SaslScram256 { username, password } => {
            config.set("security.protocol", "SASL_PLAINTEXT");
            config.set("sasl.mechanism", "SCRAM-SHA-256");
            config.set("sasl.username", username);
            config.set("sasl.password", password);
        }
        AuthConfig::SaslScram512 { username, password } => {
            config.set("security.protocol", "SASL_PLAINTEXT");
            config.set("sasl.mechanism", "SCRAM-SHA-512");
            config.set("sasl.username", username);
            config.set("sasl.password", password);
        }
        AuthConfig::Ssl {
            ca_cert,
            client_cert,
            client_key,
        } => {
            config.set("security.protocol", "SSL");
            if let Some(ca) = ca_cert {
                config.set("ssl.ca.location", ca);
            }
            if let Some(cert) = client_cert {
                config.set("ssl.certificate.location", cert);
            }
            if let Some(key) = client_key {
                config.set("ssl.key.location", key);
            }
        }
        AuthConfig::SaslSslPlain { username, password, ca_cert } => {
            config.set("security.protocol", "SASL_SSL");
            config.set("sasl.mechanism", "PLAIN");
            config.set("sasl.username", username);
            config.set("sasl.password", password);
            if let Some(ca) = ca_cert {
                config.set("ssl.ca.location", ca);
            }
        }
        AuthConfig::SaslSslScram256 { username, password, ca_cert } => {
            config.set("security.protocol", "SASL_SSL");
            config.set("sasl.mechanism", "SCRAM-SHA-256");
            config.set("sasl.username", username);
            config.set("sasl.password", password);
            if let Some(ca) = ca_cert {
                config.set("ssl.ca.location", ca);
            }
        }
        AuthConfig::SaslSslScram512 { username, password, ca_cert } => {
            config.set("security.protocol", "SASL_SSL");
            config.set("sasl.mechanism", "SCRAM-SHA-512");
            config.set("sasl.username", username);
            config.set("sasl.password", password);
            if let Some(ca) = ca_cert {
                config.set("ssl.ca.location", ca);
            }
        }
    }

    config
}

fn extract_metadata(metadata: &Metadata) -> (KafkaResponse, Vec<TopicInfo>) {
    let brokers_online: Vec<i32> = metadata.brokers().iter().map(|b| b.id()).collect();
    let broker_count = brokers_online.len();

    let mut topic_count = 0;
    let mut partition_count = 0;
    let mut topics = Vec::new();

    for topic in metadata.topics() {
        if topic.name().starts_with("__") {
            continue;
        }
        topic_count += 1;
        let partitions: Vec<PartitionInfo> = topic
            .partitions()
            .iter()
            .map(|p| {
                partition_count += 1;
                PartitionInfo {
                    id: p.id(),
                    leader: p.leader(),
                    replicas: p.replicas().to_vec(),
                    isr: p.isr().to_vec(),
                }
            })
            .collect();

        let replication_factor = partitions.first().map(|p| p.replicas.len()).unwrap_or(0);

        topics.push(TopicInfo {
            name: topic.name().to_string(),
            partitions: partitions.len(),
            replication_factor,
            partition_details: partitions,
        });
    }

    topics.sort_by(|a, b| a.name.cmp(&b.name));

    let controller_id = metadata.brokers().iter().find(|b| b.id() >= 0).map(|b| b.id());

    let resp = KafkaResponse::MetadataUpdate {
        controller_id,
        broker_count,
        brokers_online,
        topic_count,
        partition_count,
    };

    (resp, topics)
}

/// Spawns the Kafka backend task. Returns channels for communication.
pub fn spawn_kafka_backend() -> (mpsc::UnboundedSender<KafkaCommand>, mpsc::UnboundedReceiver<KafkaResponse>) {
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<KafkaCommand>();
    let (resp_tx, resp_rx) = mpsc::unbounded_channel::<KafkaResponse>();

    tokio::spawn(async move {
        let mut admin: Option<AdminClient<DefaultClientContext>> = None;
        let mut consumer: Option<BaseConsumer> = None;
        let mut producer: Option<FutureProducer> = None;
        let mut _current_config: Option<ClusterConfig> = None;
        let mut consuming = false;
        let mut _consume_topic: Option<String> = None;

        loop {
            // If consuming, poll for messages with a short timeout
            if consuming {
                if let Some(ref cons) = consumer {
                    match cons.poll(Duration::from_millis(100)) {
                        Some(Ok(msg)) => {
                            use rdkafka::Message;
                            let key = msg.key().map(|k| String::from_utf8_lossy(k).to_string());
                            let value = msg
                                .payload()
                                .map(|v| String::from_utf8_lossy(v).to_string())
                                .unwrap_or_default();
                            let timestamp = match msg.timestamp() {
                                rdkafka::Timestamp::CreateTime(ts) => Some(ts),
                                rdkafka::Timestamp::LogAppendTime(ts) => Some(ts),
                                _ => None,
                            };
                            let headers = if let Some(hdrs) = msg.headers() {
                                (0..hdrs.count())
                                    .filter_map(|i| {
                                        hdrs.get_as::<[u8]>(i).ok().map(|h| {
                                            (
                                                h.key.to_string(),
                                                String::from_utf8_lossy(h.value.unwrap_or(b""))
                                                    .to_string(),
                                            )
                                        })
                                    })
                                    .collect()
                            } else {
                                Vec::new()
                            };

                            let kafka_msg = KafkaMessage {
                                partition: msg.partition(),
                                offset: msg.offset(),
                                key,
                                value,
                                timestamp,
                                headers,
                            };
                            let _ = resp_tx.send(KafkaResponse::Messages(vec![kafka_msg]));
                        }
                        Some(Err(e)) => {
                            let _ = resp_tx.send(KafkaResponse::Error(format!("Consumer error: {}", e)));
                        }
                        None => {}
                    }
                }
            }

            // Check for commands (non-blocking if consuming, blocking otherwise)
            let cmd = if consuming {
                match cmd_rx.try_recv() {
                    Ok(cmd) => Some(cmd),
                    Err(_) => {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        continue;
                    }
                }
            } else {
                cmd_rx.recv().await
            };

            let Some(cmd) = cmd else { break };

            match cmd {
                KafkaCommand::Connect(cluster) => {
                    match build_client_config(&cluster).create::<AdminClient<DefaultClientContext>>() {
                        Ok(adm) => {
                            // Test connection by fetching metadata
                            match adm.inner().fetch_metadata(None, Duration::from_secs(10)) {
                                Ok(_meta) => {
                                    let cons: Result<BaseConsumer, _> = build_client_config(&cluster)
                                        .set("group.id", "kafka-eye-browser")
                                        .set("enable.auto.commit", "false")
                                        .set("auto.offset.reset", "latest")
                                        .create();

                                    let prod: Result<FutureProducer, _> =
                                        build_client_config(&cluster).create();

                                    match (cons, prod) {
                                        (Ok(c), Ok(p)) => {
                                            admin = Some(adm);
                                            consumer = Some(c);
                                            producer = Some(p);
                                            _current_config = Some(cluster.clone());
                                            let _ = resp_tx.send(KafkaResponse::Connected(cluster.name));
                                        }
                                        (Err(e), _) | (_, Err(e)) => {
                                            let _ = resp_tx.send(KafkaResponse::ConnectionFailed(
                                                format!("Failed to create client: {}", e),
                                            ));
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = resp_tx.send(KafkaResponse::ConnectionFailed(format!(
                                        "Cannot reach brokers: {}",
                                        e
                                    )));
                                }
                            }
                        }
                        Err(e) => {
                            let _ = resp_tx.send(KafkaResponse::ConnectionFailed(format!(
                                "Config error: {}",
                                e
                            )));
                        }
                    }
                }

                KafkaCommand::Disconnect => {
                    admin = None;
                    consumer = None;
                    producer = None;
                    _current_config = None;
                    consuming = false;
                    _consume_topic = None;
                    let _ = resp_tx.send(KafkaResponse::Disconnected);
                }

                KafkaCommand::TestConnection(cluster) => {
                    let name = cluster.name.clone();
                    match build_client_config(&cluster).create::<AdminClient<DefaultClientContext>>() {
                        Ok(adm) => {
                            match adm.inner().fetch_metadata(None, Duration::from_secs(10)) {
                                Ok(meta) => {
                                    let _ = resp_tx.send(KafkaResponse::ConnectionTestResult {
                                        cluster_name: name,
                                        success: true,
                                        message: format!(
                                            "Connected! {} brokers, {} topics",
                                            meta.brokers().len(),
                                            meta.topics().iter().filter(|t| !t.name().starts_with("__")).count()
                                        ),
                                    });
                                }
                                Err(e) => {
                                    let _ = resp_tx.send(KafkaResponse::ConnectionTestResult {
                                        cluster_name: name,
                                        success: false,
                                        message: format!("Connection failed: {}", e),
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            let _ = resp_tx.send(KafkaResponse::ConnectionTestResult {
                                cluster_name: name,
                                success: false,
                                message: format!("Config error: {}", e),
                            });
                        }
                    }
                }

                KafkaCommand::FetchMetadata => {
                    if let Some(ref adm) = admin {
                        match adm.inner().fetch_metadata(None, Duration::from_secs(10)) {
                            Ok(meta) => {
                                let (resp, _) = extract_metadata(&meta);
                                let _ = resp_tx.send(resp);
                            }
                            Err(e) => {
                                let _ = resp_tx.send(KafkaResponse::Error(format!(
                                    "Metadata fetch failed: {}",
                                    e
                                )));
                            }
                        }
                    }
                }

                KafkaCommand::FetchTopics => {
                    if let Some(ref adm) = admin {
                        match adm.inner().fetch_metadata(None, Duration::from_secs(10)) {
                            Ok(meta) => {
                                let (_, topics) = extract_metadata(&meta);
                                let _ = resp_tx.send(KafkaResponse::TopicList(topics));
                            }
                            Err(e) => {
                                let _ = resp_tx.send(KafkaResponse::Error(format!(
                                    "Topic fetch failed: {}",
                                    e
                                )));
                            }
                        }
                    }
                }

                KafkaCommand::FetchTopicDetail(topic_name) => {
                    if let Some(ref adm) = admin {
                        match adm
                            .inner()
                            .fetch_metadata(Some(&topic_name), Duration::from_secs(10))
                        {
                            Ok(meta) => {
                                if let Some(topic) = meta.topics().first() {
                                    let partitions: Vec<PartitionInfo> = topic
                                        .partitions()
                                        .iter()
                                        .map(|p| PartitionInfo {
                                            id: p.id(),
                                            leader: p.leader(),
                                            replicas: p.replicas().to_vec(),
                                            isr: p.isr().to_vec(),
                                        })
                                        .collect();
                                    let _ = resp_tx.send(KafkaResponse::TopicDetail {
                                        name: topic_name,
                                        partitions,
                                    });
                                }
                            }
                            Err(e) => {
                                let _ = resp_tx.send(KafkaResponse::Error(format!(
                                    "Topic detail fetch failed: {}",
                                    e
                                )));
                            }
                        }
                    }
                }

                KafkaCommand::CreateTopic {
                    name,
                    partitions,
                    replication_factor,
                    config,
                } => {
                    if let Some(ref adm) = admin {
                        let mut new_topic =
                            NewTopic::new(&name, partitions, TopicReplication::Fixed(replication_factor));
                        for (k, v) in &config {
                            new_topic = new_topic.set(k, v);
                        }
                        let opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(10)));
                        match adm.create_topics(&[new_topic], &opts).await {
                            Ok(results) => {
                                let mut success = true;
                                for result in &results {
                                    if let Err((_, e)) = result {
                                        let _ = resp_tx
                                            .send(KafkaResponse::Error(format!("Create topic failed: {:?}", e)));
                                        success = false;
                                    }
                                }
                                if success {
                                    let _ = resp_tx.send(KafkaResponse::TopicCreated(name));
                                }
                            }
                            Err(e) => {
                                let _ = resp_tx.send(KafkaResponse::Error(format!(
                                    "Create topic failed: {}",
                                    e
                                )));
                            }
                        }
                    }
                }

                KafkaCommand::DeleteTopic(name) => {
                    if let Some(ref adm) = admin {
                        let opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(10)));
                        match adm.delete_topics(&[&name], &opts).await {
                            Ok(results) => {
                                let mut success = true;
                                for result in &results {
                                    if let Err((_, e)) = result {
                                        let _ = resp_tx
                                            .send(KafkaResponse::Error(format!("Delete topic failed: {:?}", e)));
                                        success = false;
                                    }
                                }
                                if success {
                                    let _ = resp_tx.send(KafkaResponse::TopicDeleted(name));
                                }
                            }
                            Err(e) => {
                                let _ = resp_tx.send(KafkaResponse::Error(format!(
                                    "Delete topic failed: {}",
                                    e
                                )));
                            }
                        }
                    }
                }

                KafkaCommand::StartConsuming { topic, offset_mode } => {
                    if let Some(ref cons) = consumer {
                        // Unsubscribe first
                        cons.unsubscribe();

                        let meta = cons.fetch_metadata(Some(&topic), Duration::from_secs(10));
                        if let Ok(meta) = meta {
                            if let Some(topic_meta) = meta.topics().first() {
                                let mut tpl = TopicPartitionList::new();
                                for p in topic_meta.partitions() {
                                    let offset = match &offset_mode {
                                        OffsetMode::Earliest => rdkafka::Offset::Beginning,
                                        OffsetMode::Latest => rdkafka::Offset::End,
                                        OffsetMode::Specific(o) => rdkafka::Offset::Offset(*o),
                                        OffsetMode::Timestamp(_ts) => rdkafka::Offset::End,
                                    };
                                    tpl.add_partition_offset(&topic, p.id(), offset).ok();
                                }
                                match cons.assign(&tpl) {
                                    Ok(()) => {
                                        consuming = true;
                                        _consume_topic = Some(topic);
                                    }
                                    Err(e) => {
                                        let _ = resp_tx.send(KafkaResponse::Error(format!(
                                            "Failed to assign partitions: {}",
                                            e
                                        )));
                                    }
                                }
                            }
                        }
                    }
                }

                KafkaCommand::StopConsuming => {
                    if let Some(ref cons) = consumer {
                        cons.unsubscribe();
                    }
                    consuming = false;
                    _consume_topic = None;
                }

                KafkaCommand::FetchConsumerGroups => {
                    if let Some(ref adm) = admin {
                        match adm
                            .inner()
                            .fetch_group_list(None, Duration::from_secs(10))
                        {
                            Ok(group_list) => {
                                let groups: Vec<ConsumerGroupInfo> = group_list
                                    .groups()
                                    .iter()
                                    .filter(|g| g.name() != "kafka-eye-browser")
                                    .map(|g| ConsumerGroupInfo {
                                        name: g.name().to_string(),
                                        state: g.state().to_string(),
                                        members: g.members().len(),
                                        topics: Vec::new(),
                                        lag: Vec::new(),
                                        total_lag: 0,
                                    })
                                    .collect();
                                let _ = resp_tx.send(KafkaResponse::ConsumerGroupList(groups));
                            }
                            Err(e) => {
                                let _ = resp_tx.send(KafkaResponse::Error(format!(
                                    "Fetch groups failed: {}",
                                    e
                                )));
                            }
                        }
                    }
                }

                KafkaCommand::FetchConsumerGroupDetail(group_id) => {
                    // For detailed group info with lag, we need more complex logic
                    if let Some(ref adm) = admin {
                        match adm
                            .inner()
                            .fetch_group_list(Some(&group_id), Duration::from_secs(10))
                        {
                            Ok(group_list) => {
                                if let Some(g) = group_list.groups().first() {
                                    let info = ConsumerGroupInfo {
                                        name: g.name().to_string(),
                                        state: g.state().to_string(),
                                        members: g.members().len(),
                                        topics: Vec::new(),
                                        lag: Vec::new(),
                                        total_lag: 0,
                                    };
                                    let _ = resp_tx.send(KafkaResponse::ConsumerGroupDetail(info));
                                }
                            }
                            Err(e) => {
                                let _ = resp_tx.send(KafkaResponse::Error(format!(
                                    "Fetch group detail failed: {}",
                                    e
                                )));
                            }
                        }
                    }
                }

                KafkaCommand::ProduceMessage {
                    topic,
                    key,
                    value,
                    headers,
                } => {
                    if let Some(ref prod) = producer {
                        let mut record = FutureRecord::to(&topic).payload(&value);
                        let key_str;
                        if let Some(ref k) = key {
                            key_str = k.clone();
                            record = record.key(&key_str);
                        }

                        let mut owned_headers = rdkafka::message::OwnedHeaders::new();
                        for (k, v) in &headers {
                            owned_headers = owned_headers.insert(rdkafka::message::Header {
                                key: k,
                                value: Some(v.as_bytes()),
                            });
                        }
                        record = record.headers(owned_headers);

                        match prod.send(record, Duration::from_secs(5)).await {
                            Ok((partition, offset)) => {
                                let _ = resp_tx.send(KafkaResponse::MessageProduced {
                                    topic,
                                    partition,
                                    offset,
                                });
                            }
                            Err((e, _)) => {
                                let _ = resp_tx.send(KafkaResponse::Error(format!(
                                    "Produce failed: {}",
                                    e
                                )));
                            }
                        }
                    } else {
                        let _ = resp_tx.send(KafkaResponse::Error("No producer available".to_string()));
                    }
                }
            }
        }
    });

    (cmd_tx, resp_rx)
}
