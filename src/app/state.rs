use std::collections::HashMap;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::config::Config;
use super::commands::Command;

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    Insert,
    Command,
    Visual,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Dashboard,
    TopicList,
    TopicDetail,
    MessageProducer,
    MessageConsumer,
    ConsumerGroups,
    Monitoring,
    Settings,
}

#[derive(Debug, Clone)]
pub struct TopicInfo {
    pub name: String,
    pub partitions: u32,
    pub replicas: u16,
    pub configs: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub topic: String,
    pub partition: i32,
    pub offset: i64,
    pub key: Option<String>,
    pub value: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ConsumerGroupInfo {
    pub name: String,
    pub state: String,
    pub protocol: String,
    pub members: Vec<ConsumerMember>,
}

#[derive(Debug, Clone)]
pub struct ConsumerMember {
    pub id: String,
    pub client_id: String,
    pub host: String,
    pub assignments: Vec<TopicPartition>,
}

#[derive(Debug, Clone)]
pub struct TopicPartition {
    pub topic: String,
    pub partition: i32,
}

pub struct AppState {
    pub mode: AppMode,
    pub current_screen: Screen,
    pub last_key: Option<char>,
    
    // Input handling
    pub command_input: String,
    pub input_buffer: String,
    pub search_query: String,
    
    // Navigation
    pub selected_index: usize,
    pub scroll_offset: usize,
    
    // Data
    pub topics: Vec<TopicInfo>,
    pub messages: Vec<Message>,
    pub consumer_groups: Vec<ConsumerGroupInfo>,
    pub selected_topic: Option<String>,
    pub selected_consumer_group: Option<String>,
    
    // Connection info
    pub connected: bool,
    pub current_cluster: Option<String>,
    pub connection_status: String,
    
    // Monitoring
    pub stats: MonitoringStats,
}

#[derive(Debug, Clone, Default)]
pub struct MonitoringStats {
    pub total_topics: u32,
    pub total_partitions: u32,
    pub total_consumer_groups: u32,
    pub messages_per_sec: f64,
    pub bytes_per_sec: f64,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            mode: AppMode::Normal,
            current_screen: Screen::Dashboard,
            last_key: None,
            
            command_input: String::new(),
            input_buffer: String::new(),
            search_query: String::new(),
            
            selected_index: 0,
            scroll_offset: 0,
            
            topics: Vec::new(),
            messages: Vec::new(),
            consumer_groups: Vec::new(),
            selected_topic: None,
            selected_consumer_group: None,
            
            connected: false,
            current_cluster: None,
            connection_status: "Disconnected".to_string(),
            
            stats: MonitoringStats::default(),
        }
    }

    // Navigation methods
    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.adjust_scroll();
        }
    }

    pub fn move_down(&mut self) {
        let max_index = self.get_max_index();
        if self.selected_index < max_index {
            self.selected_index += 1;
            self.adjust_scroll();
        }
    }

    pub fn move_left(&mut self) {
        // Handle horizontal navigation if needed
    }

    pub fn move_right(&mut self) {
        // Handle horizontal navigation if needed
    }

    pub fn go_to_top(&mut self) {
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    pub fn go_to_bottom(&mut self) {
        self.selected_index = self.get_max_index();
        self.adjust_scroll();
    }

    pub fn page_up(&mut self) {
        let page_size = 10; // This should be based on terminal height
        self.selected_index = self.selected_index.saturating_sub(page_size);
        self.adjust_scroll();
    }

    pub fn page_down(&mut self) {
        let page_size = 10; // This should be based on terminal height
        let max_index = self.get_max_index();
        self.selected_index = (self.selected_index + page_size).min(max_index);
        self.adjust_scroll();
    }

    // Screen navigation
    pub fn next_screen(&mut self) {
        self.current_screen = match self.current_screen {
            Screen::Dashboard => Screen::TopicList,
            Screen::TopicList => Screen::MessageProducer,
            Screen::MessageProducer => Screen::MessageConsumer,
            Screen::MessageConsumer => Screen::ConsumerGroups,
            Screen::ConsumerGroups => Screen::Monitoring,
            Screen::Monitoring => Screen::Settings,
            Screen::Settings => Screen::Dashboard,
            Screen::TopicDetail => Screen::TopicList,
        };
        self.reset_selection();
    }

    pub fn previous_screen(&mut self) {
        self.current_screen = match self.current_screen {
            Screen::Dashboard => Screen::Settings,
            Screen::TopicList => Screen::Dashboard,
            Screen::MessageProducer => Screen::TopicList,
            Screen::MessageConsumer => Screen::MessageProducer,
            Screen::ConsumerGroups => Screen::MessageConsumer,
            Screen::Monitoring => Screen::ConsumerGroups,
            Screen::Settings => Screen::Monitoring,
            Screen::TopicDetail => Screen::TopicList,
        };
        self.reset_selection();
    }

    // Helper methods
    fn get_max_index(&self) -> usize {
        match self.current_screen {
            Screen::TopicList => self.topics.len().saturating_sub(1),
            Screen::MessageConsumer | Screen::MessageProducer => self.messages.len().saturating_sub(1),
            Screen::ConsumerGroups => self.consumer_groups.len().saturating_sub(1),
            _ => 0,
        }
    }

    fn adjust_scroll(&mut self) {
        let visible_items = 20; // This should be based on terminal height
        
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + visible_items {
            self.scroll_offset = self.selected_index.saturating_sub(visible_items - 1);
        }
    }

    fn reset_selection(&mut self) {
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    // Data management
    pub fn add_topic(&mut self, topic: TopicInfo) {
        self.topics.push(topic);
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        // Keep only the last 1000 messages to prevent memory issues
        if self.messages.len() > 1000 {
            self.messages.remove(0);
        }
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    pub fn get_selected_topic(&self) -> Option<&TopicInfo> {
        if self.current_screen == Screen::TopicList && self.selected_index < self.topics.len() {
            Some(&self.topics[self.selected_index])
        } else {
            None
        }
    }

    pub fn get_selected_consumer_group(&self) -> Option<&ConsumerGroupInfo> {
        if self.current_screen == Screen::ConsumerGroups && self.selected_index < self.consumer_groups.len() {
            Some(&self.consumer_groups[self.selected_index])
        } else {
            None
        }
    }

    // Status methods
    pub fn set_connected(&mut self, connected: bool, cluster: Option<String>) {
        self.connected = connected;
        self.current_cluster = cluster;
        self.connection_status = if connected {
            format!("Connected to {}", self.current_cluster.as_deref().unwrap_or("Unknown"))
        } else {
            "Disconnected".to_string()
        };
    }

    pub fn update_stats(&mut self, stats: MonitoringStats) {
        self.stats = stats;
    }

    // Cluster management
    pub fn handle_command(&mut self, command: Command, config: &mut Config) -> Result<()> {
        match command {
            Command::AddCluster { name, brokers, client_id, security } => {
                config.add_cluster(&name, &brokers, &client_id, security)?;
                config.save("config.yaml")?;
                self.set_status_message(&format!("Added cluster {}", name));
                Ok(())
            }
            Command::RemoveCluster { name } => {
                if let Some(current) = &self.current_cluster {
                    if current == &name {
                        self.set_connected(false, None);
                    }
                }
                config.remove_cluster(&name)?;
                config.save("config.yaml")?;
                self.set_status_message(&format!("Removed cluster {}", name));
                Ok(())
            }
            Command::SwitchCluster { name } => {
                if config.has_cluster(&name) {
                    config.set_active_cluster(&name)?;
                    config.save("config.yaml")?;
                    // The actual connection will be handled by the main loop
                    self.current_cluster = Some(name.clone());
                    self.set_status_message(&format!("Switching to cluster {}", name));
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Cluster {} not found", name))
                }
            }
            Command::ListClusters => {
                let clusters = config.list_clusters();
                let message = if clusters.is_empty() {
                    "No clusters configured".to_string()
                } else {
                    format!("Configured clusters: {}", clusters.join(", "))
                };
                self.set_status_message(&message);
                Ok(())
            }
            Command::Quit => Ok(()),
            Command::Connect => {
                // Connection is handled in App, just acknowledge here
                self.set_status_message("Connection command received");
                Ok(())
            }
            Command::Disconnect => {
                // Disconnection is handled in App, just acknowledge here  
                self.set_status_message("Disconnection command received");
                Ok(())
            }
            Command::Unknown(msg) => Err(anyhow::anyhow!("Unknown command: {}", msg)),
        }
    }

    fn set_status_message(&mut self, msg: &str) {
        self.connection_status = msg.to_string();
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
