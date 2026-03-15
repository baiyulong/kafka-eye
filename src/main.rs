mod app;
mod config;
mod event;
mod kafka;
mod tui;
mod ui;

use app::*;
use config::AppConfig;
use crossterm::event::{KeyCode, KeyEvent};
use event::Event;
use kafka::client::{KafkaCommand, KafkaResponse};
use std::collections::HashMap;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::load()?;
    let mut app = App::new(config);
    let mut terminal = tui::init()?;
    let mut events = event::EventHandler::new(100);

    let (kafka_tx, mut kafka_rx) = kafka::client::spawn_kafka_backend();

    let result = run_app(&mut terminal, &mut app, &mut events, &kafka_tx, &mut kafka_rx).await;

    tui::restore(&mut terminal)?;
    result
}

async fn run_app(
    terminal: &mut tui::Terminal,
    app: &mut App,
    events: &mut event::EventHandler,
    kafka_tx: &mpsc::UnboundedSender<KafkaCommand>,
    kafka_rx: &mut mpsc::UnboundedReceiver<KafkaResponse>,
) -> anyhow::Result<()> {
    while app.running {
        terminal.draw(|frame| ui::render(app, frame))?;

        // Process any pending Kafka responses (non-blocking)
        while let Ok(response) = kafka_rx.try_recv() {
            handle_kafka_response(app, response);
        }

        // Wait for next event
        match events.next().await? {
            Event::Key(key) => handle_key_event(app, key, kafka_tx),
            Event::Tick => {
                // Periodic tasks could go here
            }
            Event::Resize(_, _) => {
                // Terminal will auto-resize on next draw
            }
        }
    }

    Ok(())
}

fn handle_kafka_response(app: &mut App, response: KafkaResponse) {
    match response {
        KafkaResponse::Connected(name) => {
            app.log_info(&format!("Connected to cluster: {}", name));
            app.route = Route::Dashboard;
            app.focus = Focus::Sidebar;
        }
        KafkaResponse::Disconnected => {
            app.log_info("Disconnected from cluster");
            app.route = Route::ClusterSelect;
            app.active_cluster = None;
        }
        KafkaResponse::ConnectionFailed(msg) => {
            app.log_error(&format!("Connection failed: {}", msg));
        }
        KafkaResponse::MetadataUpdate {
            controller_id,
            broker_count,
            brokers_online,
            topic_count,
            partition_count,
        } => {
            app.dashboard = DashboardState {
                controller_id,
                broker_count,
                brokers_online,
                topic_count,
                partition_count,
                loading: false,
            };
            app.log_info("Metadata refreshed");
        }
        KafkaResponse::TopicList(topics) => {
            app.topics.topics = topics;
            app.topics.loading = false;
            app.log_info(&format!("Loaded {} topics", app.topics.topics.len()));
        }
        KafkaResponse::TopicDetail { name, partitions } => {
            app.topic_detail = TopicDetailState {
                topic_name: name.clone(),
                partitions,
                config: Vec::new(),
                selected_partition: 0,
            };
            app.log_info(&format!("Loaded detail for topic: {}", name));
        }
        KafkaResponse::TopicCreated(name) => {
            app.log_info(&format!("Topic '{}' created successfully", name));
            app.dialog = None;
        }
        KafkaResponse::TopicDeleted(name) => {
            app.log_info(&format!("Topic '{}' deleted successfully", name));
            app.dialog = None;
        }
        KafkaResponse::Messages(msgs) => {
            for msg in msgs {
                app.messages.messages.push(msg);
            }
            // Keep only last 1000 messages
            if app.messages.messages.len() > 1000 {
                let drain_count = app.messages.messages.len() - 1000;
                app.messages.messages.drain(0..drain_count);
            }
            if app.messages.auto_scroll {
                let len = app.messages.filtered_messages().len();
                if len > 0 {
                    app.messages.selected = len - 1;
                }
            }
        }
        KafkaResponse::ConsumerGroupList(groups) => {
            app.consumer_groups.groups = groups;
            app.consumer_groups.loading = false;
            app.log_info(&format!("Loaded {} consumer groups", app.consumer_groups.groups.len()));
        }
        KafkaResponse::ConsumerGroupDetail(info) => {
            // Update the group in the list
            if let Some(existing) = app.consumer_groups.groups.iter_mut().find(|g| g.name == info.name) {
                *existing = info;
            }
        }
        KafkaResponse::MessageProduced {
            topic,
            partition,
            offset,
        } => {
            if let Some(Dialog::ProduceMessage(ref mut d)) = app.dialog {
                d.result_message = Some(format!(
                    "✓ Sent to {}[{}] at offset {}",
                    topic, partition, offset
                ));
            }
            app.log_info(&format!("Message produced to {}[{}]@{}", topic, partition, offset));
        }
        KafkaResponse::Error(msg) => {
            app.log_error(&msg);
        }
        KafkaResponse::ConnectionTestResult {
            cluster_name,
            success,
            message,
        } => {
            if let Some(Dialog::ConnectionTest(ref mut d)) = app.dialog {
                d.status = if success {
                    ConnectionTestStatus::Success(message)
                } else {
                    ConnectionTestStatus::Failed(message)
                };
            }
            if success {
                app.log_info(&format!("Connection test OK: {}", cluster_name));
            } else {
                app.log_error(&format!("Connection test failed: {}", cluster_name));
            }
        }
    }
}

fn handle_key_event(app: &mut App, key: KeyEvent, kafka_tx: &mpsc::UnboundedSender<KafkaCommand>) {
    // Global shortcuts
    if key.code == KeyCode::Char('?') && app.dialog.is_none() && app.focus != Focus::Search {
        app.show_help = !app.show_help;
        return;
    }

    if app.show_help {
        if key.code == KeyCode::Esc || key.code == KeyCode::Char('?') {
            app.show_help = false;
        }
        return;
    }

    // Dialog handling
    if app.dialog.is_some() {
        handle_dialog_key(app, key, kafka_tx);
        return;
    }

    // Search mode
    if app.focus == Focus::Search {
        handle_search_key(app, key);
        return;
    }

    match &app.route {
        Route::ClusterSelect => handle_cluster_select_key(app, key, kafka_tx),
        Route::Dashboard => handle_main_key(app, key, kafka_tx),
        Route::Topics => handle_main_key(app, key, kafka_tx),
        Route::TopicDetail(_) => handle_main_key(app, key, kafka_tx),
        Route::Messages(_) => handle_main_key(app, key, kafka_tx),
        Route::ConsumerGroups => handle_main_key(app, key, kafka_tx),
        Route::ConsumerGroupDetail(_) => handle_main_key(app, key, kafka_tx),
    }
}

fn handle_cluster_select_key(
    app: &mut App,
    key: KeyEvent,
    kafka_tx: &mpsc::UnboundedSender<KafkaCommand>,
) {
    match key.code {
        KeyCode::Char('q') => app.running = false,
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.config.clusters.is_empty() {
                app.cluster_select_index = (app.cluster_select_index + 1) % app.config.clusters.len();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !app.config.clusters.is_empty() {
                app.cluster_select_index = (app.cluster_select_index + app.config.clusters.len() - 1) % app.config.clusters.len();
            }
        }
        KeyCode::Enter => {
            if let Some(cluster) = app.config.clusters.get(app.cluster_select_index).cloned() {
                app.active_cluster = Some(app.cluster_select_index);
                app.log_info(&format!("Connecting to {}...", cluster.name));
                let _ = kafka_tx.send(KafkaCommand::Connect(cluster));
            }
        }
        KeyCode::Char('a') => {
            app.dialog = Some(Dialog::EditCluster(EditClusterDialog::default()));
        }
        KeyCode::Char('e') => {
            if let Some(cluster) = app.config.clusters.get(app.cluster_select_index) {
                app.dialog = Some(Dialog::EditCluster(EditClusterDialog::from_config(cluster, app.cluster_select_index)));
            }
        }
        KeyCode::Char('d') => {
            if let Some(cluster) = app.config.clusters.get(app.cluster_select_index) {
                app.dialog = Some(Dialog::DeleteConfirm(DeleteConfirmDialog::new("cluster", &cluster.name)));
            }
        }
        KeyCode::Char('t') => {
            if let Some(cluster) = app.config.clusters.get(app.cluster_select_index) {
                app.dialog = Some(Dialog::ConnectionTest(ConnectionTestDialog {
                    cluster_name: cluster.name.clone(),
                    status: ConnectionTestStatus::Testing,
                }));
                let _ = kafka_tx.send(KafkaCommand::TestConnection(cluster.clone()));
            }
        }
        _ => {}
    }
}

fn handle_main_key(app: &mut App, key: KeyEvent, kafka_tx: &mpsc::UnboundedSender<KafkaCommand>) {
    match key.code {
        KeyCode::Char('q') => {
            let _ = kafka_tx.send(KafkaCommand::StopConsuming);
            let _ = kafka_tx.send(KafkaCommand::Disconnect);
            app.running = false;
        }
        KeyCode::Tab => {
            app.focus = match app.focus {
                Focus::Sidebar => Focus::Content,
                Focus::Content => Focus::Sidebar,
                _ => Focus::Content,
            };
        }
        KeyCode::Char('/') => {
            app.focus = Focus::Search;
        }
        _ => {
            if app.focus == Focus::Sidebar {
                handle_sidebar_key(app, key, kafka_tx);
            } else {
                handle_content_key(app, key, kafka_tx);
            }
        }
    }
}

fn handle_sidebar_key(
    app: &mut App,
    key: KeyEvent,
    kafka_tx: &mpsc::UnboundedSender<KafkaCommand>,
) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => app.sidebar.next(),
        KeyCode::Char('k') | KeyCode::Up => app.sidebar.previous(),
        KeyCode::Enter => {
            if let Some(route) = app.sidebar.current_route().cloned() {
                app.navigate(route.clone());
                // Trigger data fetch on navigation
                match &route {
                    Route::Dashboard => {
                        let _ = kafka_tx.send(KafkaCommand::FetchMetadata);
                    }
                    Route::Topics => {
                        let _ = kafka_tx.send(KafkaCommand::FetchTopics);
                    }
                    Route::ConsumerGroups => {
                        let _ = kafka_tx.send(KafkaCommand::FetchConsumerGroups);
                    }
                    _ => {}
                }
            }
        }
        KeyCode::Esc => {
            let _ = kafka_tx.send(KafkaCommand::StopConsuming);
            let _ = kafka_tx.send(KafkaCommand::Disconnect);
            app.route = Route::ClusterSelect;
            app.active_cluster = None;
            app.focus = Focus::Content;
        }
        _ => {}
    }
}

fn handle_content_key(
    app: &mut App,
    key: KeyEvent,
    kafka_tx: &mpsc::UnboundedSender<KafkaCommand>,
) {
    match &app.route.clone() {
        Route::Dashboard => match key.code {
            KeyCode::Char('r') => {
                let _ = kafka_tx.send(KafkaCommand::FetchMetadata);
                app.log_info("Refreshing metadata...");
            }
            _ => {}
        },
        Route::Topics => match key.code {
            KeyCode::Char('j') | KeyCode::Down => app.topics.next(),
            KeyCode::Char('k') | KeyCode::Up => app.topics.previous(),
            KeyCode::Char('r') => {
                let _ = kafka_tx.send(KafkaCommand::FetchTopics);
                app.log_info("Refreshing topics...");
            }
            KeyCode::Char('c') => {
                app.dialog = Some(Dialog::CreateTopic(CreateTopicDialog::default()));
            }
            KeyCode::Char('d') => {
                let filtered = app.topics.filtered_topics();
                if let Some(topic) = filtered.get(app.topics.selected) {
                    app.dialog = Some(Dialog::DeleteConfirm(DeleteConfirmDialog::new("topic", &topic.name)));
                }
            }
            KeyCode::Enter => {
                let filtered = app.topics.filtered_topics();
                if let Some(topic) = filtered.get(app.topics.selected) {
                    let name = topic.name.clone();
                    app.navigate(Route::TopicDetail(name.clone()));
                    let _ = kafka_tx.send(KafkaCommand::FetchTopicDetail(name));
                }
            }
            KeyCode::Char('m') => {
                let filtered = app.topics.filtered_topics();
                if let Some(topic) = filtered.get(app.topics.selected) {
                    let name = topic.name.clone();
                    app.messages = MessageState::new(&name);
                    app.navigate(Route::Messages(name));
                }
            }
            _ => {}
        },
        Route::TopicDetail(_) => match key.code {
            KeyCode::Esc => {
                app.navigate(Route::Topics);
                let _ = kafka_tx.send(KafkaCommand::FetchTopics);
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let len = app.topic_detail.partitions.len();
                if len > 0 {
                    app.topic_detail.selected_partition = (app.topic_detail.selected_partition + 1) % len;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let len = app.topic_detail.partitions.len();
                if len > 0 {
                    app.topic_detail.selected_partition =
                        (app.topic_detail.selected_partition + len - 1) % len;
                }
            }
            KeyCode::Char('m') => {
                let name = app.topic_detail.topic_name.clone();
                app.messages = MessageState::new(&name);
                app.navigate(Route::Messages(name));
            }
            KeyCode::Char('r') => {
                let name = app.topic_detail.topic_name.clone();
                let _ = kafka_tx.send(KafkaCommand::FetchTopicDetail(name));
                app.log_info("Refreshing topic detail...");
            }
            _ => {}
        },
        Route::Messages(topic) => {
            let topic = topic.clone();
            match key.code {
                KeyCode::Esc => {
                    if app.messages.show_detail {
                        app.messages.show_detail = false;
                    } else {
                        let _ = kafka_tx.send(KafkaCommand::StopConsuming);
                        app.messages.consuming = false;
                        app.navigate(Route::Topics);
                        let _ = kafka_tx.send(KafkaCommand::FetchTopics);
                    }
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    app.messages.auto_scroll = false;
                    app.messages.next();
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    app.messages.auto_scroll = false;
                    app.messages.previous();
                }
                KeyCode::Char('s') => {
                    if app.messages.consuming {
                        let _ = kafka_tx.send(KafkaCommand::StopConsuming);
                        app.messages.consuming = false;
                        app.log_info("Stopped consuming");
                    } else {
                        app.messages.consuming = true;
                        app.messages.auto_scroll = true;
                        let _ = kafka_tx.send(KafkaCommand::StartConsuming {
                            topic: topic.clone(),
                            offset_mode: app.messages.offset_mode.clone(),
                        });
                        app.log_info("Started consuming");
                    }
                }
                KeyCode::Enter => {
                    app.messages.show_detail = !app.messages.show_detail;
                }
                KeyCode::Char('p') => {
                    app.dialog = Some(Dialog::ProduceMessage(ProduceMessageDialog::new(&topic)));
                }
                KeyCode::Char('1') => {
                    app.messages.offset_mode = OffsetMode::Earliest;
                    app.messages.messages.clear();
                    app.messages.consuming = true;
                    app.messages.auto_scroll = true;
                    let _ = kafka_tx.send(KafkaCommand::StopConsuming);
                    let _ = kafka_tx.send(KafkaCommand::StartConsuming {
                        topic: topic.clone(),
                        offset_mode: OffsetMode::Earliest,
                    });
                    app.log_info("Consuming from earliest...");
                }
                KeyCode::Char('2') => {
                    app.messages.offset_mode = OffsetMode::Latest;
                    app.messages.messages.clear();
                    app.messages.consuming = true;
                    app.messages.auto_scroll = true;
                    let _ = kafka_tx.send(KafkaCommand::StopConsuming);
                    let _ = kafka_tx.send(KafkaCommand::StartConsuming {
                        topic: topic.clone(),
                        offset_mode: OffsetMode::Latest,
                    });
                    app.log_info("Consuming from latest...");
                }
                _ => {}
            }
        }
        Route::ConsumerGroups => match key.code {
            KeyCode::Char('j') | KeyCode::Down => app.consumer_groups.next(),
            KeyCode::Char('k') | KeyCode::Up => app.consumer_groups.previous(),
            KeyCode::Char('r') => {
                let _ = kafka_tx.send(KafkaCommand::FetchConsumerGroups);
                app.log_info("Refreshing consumer groups...");
            }
            KeyCode::Enter => {
                if let Some(group) = app.consumer_groups.groups.get(app.consumer_groups.selected) {
                    let name = group.name.clone();
                    app.navigate(Route::ConsumerGroupDetail(name.clone()));
                    let _ = kafka_tx.send(KafkaCommand::FetchConsumerGroupDetail(name));
                }
            }
            _ => {}
        },
        Route::ConsumerGroupDetail(_) => match key.code {
            KeyCode::Esc => {
                app.navigate(Route::ConsumerGroups);
                let _ = kafka_tx.send(KafkaCommand::FetchConsumerGroups);
            }
            KeyCode::Char('r') => {
                if let Some(group) = app.consumer_groups.groups.get(app.consumer_groups.selected) {
                    let name = group.name.clone();
                    let _ = kafka_tx.send(KafkaCommand::FetchConsumerGroupDetail(name));
                    app.log_info("Refreshing group detail...");
                }
            }
            _ => {}
        },
        _ => {}
    }
}

fn handle_search_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Enter => {
            app.focus = Focus::Content;
        }
        KeyCode::Char(c) => {
            match &app.route {
                Route::Topics => app.topics.search_query.push(c),
                Route::Messages(_) => app.messages.search_query.push(c),
                _ => {}
            }
        }
        KeyCode::Backspace => {
            match &app.route {
                Route::Topics => { app.topics.search_query.pop(); }
                Route::Messages(_) => { app.messages.search_query.pop(); }
                _ => {}
            }
        }
        _ => {}
    }
}

fn handle_dialog_key(
    app: &mut App,
    key: KeyEvent,
    kafka_tx: &mpsc::UnboundedSender<KafkaCommand>,
) {
    if key.code == KeyCode::Esc {
        app.dialog = None;
        app.focus = Focus::Content;
        return;
    }

    let dialog = app.dialog.take();
    match dialog {
        Some(Dialog::CreateTopic(mut d)) => {
            match key.code {
                KeyCode::Tab => {
                    d.focused_field = (d.focused_field + 1) % 4;
                }
                KeyCode::BackTab => {
                    d.focused_field = (d.focused_field + 3) % 4;
                }
                KeyCode::Enter => {
                    if !d.name.is_empty() {
                        let partitions = d.partitions.parse::<i32>().unwrap_or(3);
                        let replication_factor = d.replication_factor.parse::<i32>().unwrap_or(1);
                        let mut config = HashMap::new();
                        if !d.retention_ms.is_empty() {
                            config.insert("retention.ms".to_string(), d.retention_ms.clone());
                        }
                        let _ = kafka_tx.send(KafkaCommand::CreateTopic {
                            name: d.name.clone(),
                            partitions,
                            replication_factor,
                            config,
                        });
                        app.log_info(&format!("Creating topic '{}'...", d.name));
                        return; // Dialog will be closed by response
                    }
                }
                KeyCode::Char(c) => {
                    match d.focused_field {
                        0 => d.name.push(c),
                        1 => if c.is_ascii_digit() { d.partitions.push(c) },
                        2 => if c.is_ascii_digit() { d.replication_factor.push(c) },
                        3 => if c.is_ascii_digit() { d.retention_ms.push(c) },
                        _ => {}
                    }
                }
                KeyCode::Backspace => {
                    match d.focused_field {
                        0 => { d.name.pop(); }
                        1 => { d.partitions.pop(); }
                        2 => { d.replication_factor.pop(); }
                        3 => { d.retention_ms.pop(); }
                        _ => {}
                    }
                }
                _ => {}
            }
            app.dialog = Some(Dialog::CreateTopic(d));
        }
        Some(Dialog::DeleteConfirm(mut d)) => {
            match key.code {
                KeyCode::Enter => {
                    if d.confirm_input == d.item_name {
                        match d.item_type.as_str() {
                            "topic" => {
                                let _ = kafka_tx.send(KafkaCommand::DeleteTopic(d.item_name.clone()));
                                app.log_info(&format!("Deleting topic '{}'...", d.item_name));
                                // Refresh topics after deletion
                                let _ = kafka_tx.send(KafkaCommand::FetchTopics);
                            }
                            "cluster" => {
                                app.config.clusters.retain(|c| c.name != d.item_name);
                                let _ = app.config.save();
                                app.log_info(&format!("Cluster '{}' removed", d.item_name));
                                if app.cluster_select_index >= app.config.clusters.len() && !app.config.clusters.is_empty() {
                                    app.cluster_select_index = app.config.clusters.len() - 1;
                                }
                                app.dialog = None;
                                return;
                            }
                            _ => {}
                        }
                        return; // Dialog will be closed by response or already closed
                    }
                }
                KeyCode::Char(c) => {
                    d.confirm_input.push(c);
                }
                KeyCode::Backspace => {
                    d.confirm_input.pop();
                }
                _ => {}
            }
            app.dialog = Some(Dialog::DeleteConfirm(d));
        }
        Some(Dialog::ProduceMessage(mut d)) => {
            match key.code {
                KeyCode::Tab => {
                    d.focused_field = (d.focused_field + 1) % 3;
                }
                KeyCode::BackTab => {
                    d.focused_field = (d.focused_field + 2) % 3;
                }
                KeyCode::F(5) => {
                    // Send message
                    let headers: Vec<(String, String)> = if d.headers.is_empty() {
                        Vec::new()
                    } else {
                        d.headers.split(',').filter_map(|pair| {
                            let parts: Vec<&str> = pair.splitn(2, ':').collect();
                            if parts.len() == 2 {
                                Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
                            } else {
                                None
                            }
                        }).collect()
                    };
                    let _ = kafka_tx.send(KafkaCommand::ProduceMessage {
                        topic: d.topic.clone(),
                        key: if d.key.is_empty() { None } else { Some(d.key.clone()) },
                        value: d.value.clone(),
                        headers,
                    });
                    d.result_message = Some("⏳ Sending...".to_string());
                }
                KeyCode::Char(c) => {
                    match d.focused_field {
                        0 => d.key.push(c),
                        1 => d.value.push(c),
                        2 => d.headers.push(c),
                        _ => {}
                    }
                    d.result_message = None;
                }
                KeyCode::Backspace => {
                    match d.focused_field {
                        0 => { d.key.pop(); }
                        1 => { d.value.pop(); }
                        2 => { d.headers.pop(); }
                        _ => {}
                    }
                    d.result_message = None;
                }
                _ => {}
            }
            app.dialog = Some(Dialog::ProduceMessage(d));
        }
        Some(Dialog::ResetOffset(mut d)) => {
            match key.code {
                KeyCode::Left | KeyCode::Right => {
                    d.target = match d.target {
                        ResetTarget::Earliest => ResetTarget::Latest,
                        ResetTarget::Latest => ResetTarget::Earliest,
                    };
                }
                KeyCode::Enter => {
                    app.log_info(&format!("Offset reset for group '{}' is not yet implemented", d.group_id));
                    app.dialog = None;
                    return;
                }
                _ => {}
            }
            app.dialog = Some(Dialog::ResetOffset(d));
        }
        Some(Dialog::EditCluster(mut d)) => {
            let max_fields = match d.auth_type {
                1 | 2 | 3 => 5, // name, brokers, auth, user, pass
                4 => 6,         // name, brokers, auth, ca, cert, key
                _ => 3,         // name, brokers, auth
            };
            match key.code {
                KeyCode::Tab => {
                    d.focused_field = (d.focused_field + 1) % max_fields;
                }
                KeyCode::BackTab => {
                    d.focused_field = (d.focused_field + max_fields - 1) % max_fields;
                }
                KeyCode::Left => {
                    if d.focused_field == 2 {
                        d.auth_type = if d.auth_type == 0 { 4 } else { d.auth_type - 1 };
                    }
                }
                KeyCode::Right => {
                    if d.focused_field == 2 {
                        d.auth_type = (d.auth_type + 1) % 5;
                    }
                }
                KeyCode::Enter => {
                    if !d.name.is_empty() && !d.brokers.is_empty() {
                        let config = d.to_config();
                        if let Some(idx) = d.editing_index {
                            if idx < app.config.clusters.len() {
                                app.config.clusters[idx] = config;
                            }
                        } else {
                            app.config.clusters.push(config);
                        }
                        let _ = app.config.save();
                        app.log_info("Cluster configuration saved");
                        app.dialog = None;
                        return;
                    }
                }
                KeyCode::Char(c) => {
                    match d.focused_field {
                        0 => d.name.push(c),
                        1 => d.brokers.push(c),
                        2 => {} // auth type uses arrows
                        3 => match d.auth_type {
                            1 | 2 | 3 => d.username.push(c),
                            4 => d.ca_cert.push(c),
                            _ => {}
                        },
                        4 => match d.auth_type {
                            1 | 2 | 3 => d.password.push(c),
                            4 => d.client_cert.push(c),
                            _ => {}
                        },
                        5 => if d.auth_type == 4 {
                            d.client_key.push(c);
                        },
                        _ => {}
                    }
                }
                KeyCode::Backspace => {
                    match d.focused_field {
                        0 => { d.name.pop(); }
                        1 => { d.brokers.pop(); }
                        2 => {}
                        3 => match d.auth_type {
                            1 | 2 | 3 => { d.username.pop(); }
                            4 => { d.ca_cert.pop(); }
                            _ => {}
                        },
                        4 => match d.auth_type {
                            1 | 2 | 3 => { d.password.pop(); }
                            4 => { d.client_cert.pop(); }
                            _ => {}
                        },
                        5 => if d.auth_type == 4 {
                            d.client_key.pop();
                        },
                        _ => {}
                    }
                }
                _ => {}
            }
            app.dialog = Some(Dialog::EditCluster(d));
        }
        Some(Dialog::ConnectionTest(_d)) => {
            // Only Esc closes this, already handled above
            app.dialog = Some(Dialog::ConnectionTest(_d));
        }
        None => {}
    }
}
