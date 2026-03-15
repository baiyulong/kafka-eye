#![allow(dead_code)]

use crate::config::{AppConfig, ClusterConfig};

/// Navigation route
#[derive(Debug, Clone, PartialEq)]
pub enum Route {
    ClusterSelect,
    Dashboard,
    Topics,
    TopicDetail(String),
    Messages(String),
    ConsumerGroups,
    ConsumerGroupDetail(String),
}

/// Which panel has focus
#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    Sidebar,
    Content,
    Dialog,
    Search,
}

/// Active dialog/popup
#[derive(Debug, Clone)]
pub enum Dialog {
    CreateTopic(CreateTopicDialog),
    DeleteConfirm(DeleteConfirmDialog),
    ProduceMessage(ProduceMessageDialog),
    ResetOffset(ResetOffsetDialog),
    EditCluster(EditClusterDialog),
    ConnectionTest(ConnectionTestDialog),
}

#[derive(Debug, Clone)]
pub struct CreateTopicDialog {
    pub name: String,
    pub partitions: String,
    pub replication_factor: String,
    pub retention_ms: String,
    pub focused_field: usize,
}

impl Default for CreateTopicDialog {
    fn default() -> Self {
        Self {
            name: String::new(),
            partitions: "3".to_string(),
            replication_factor: "1".to_string(),
            retention_ms: String::new(),
            focused_field: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeleteConfirmDialog {
    pub item_type: String,
    pub item_name: String,
    pub confirmed: bool,
    pub confirm_input: String,
}

impl DeleteConfirmDialog {
    pub fn new(item_type: &str, item_name: &str) -> Self {
        Self {
            item_type: item_type.to_string(),
            item_name: item_name.to_string(),
            confirmed: false,
            confirm_input: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProduceMessageDialog {
    pub topic: String,
    pub key: String,
    pub value: String,
    pub headers: String,
    pub focused_field: usize,
    pub result_message: Option<String>,
}

impl ProduceMessageDialog {
    pub fn new(topic: &str) -> Self {
        Self {
            topic: topic.to_string(),
            key: String::new(),
            value: String::new(),
            headers: String::new(),
            focused_field: 0,
            result_message: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ResetTarget {
    Earliest,
    Latest,
}

#[derive(Debug, Clone)]
pub struct ResetOffsetDialog {
    pub group_id: String,
    pub topic: String,
    pub target: ResetTarget,
    pub confirmed: bool,
}

impl ResetOffsetDialog {
    pub fn new(group_id: &str, topic: &str) -> Self {
        Self {
            group_id: group_id.to_string(),
            topic: topic.to_string(),
            target: ResetTarget::Earliest,
            confirmed: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EditClusterDialog {
    pub name: String,
    pub brokers: String,
    pub auth_type: usize, // 0=None, 1=SASL/PLAIN, 2=SCRAM-256, 3=SCRAM-512, 4=SSL
    pub username: String,
    pub password: String,
    pub ca_cert: String,
    pub client_cert: String,
    pub client_key: String,
    pub focused_field: usize,
    pub editing_index: Option<usize>, // None = new, Some(i) = editing
}

impl Default for EditClusterDialog {
    fn default() -> Self {
        Self {
            name: String::new(),
            brokers: String::new(),
            auth_type: 0,
            username: String::new(),
            password: String::new(),
            ca_cert: String::new(),
            client_cert: String::new(),
            client_key: String::new(),
            focused_field: 0,
            editing_index: None,
        }
    }
}

impl EditClusterDialog {
    pub fn from_config(config: &ClusterConfig, index: usize) -> Self {
        use crate::config::AuthConfig;
        let (auth_type, username, password, ca_cert, client_cert, client_key) = match &config.auth {
            AuthConfig::None => (0, String::new(), String::new(), String::new(), String::new(), String::new()),
            AuthConfig::SaslPlain { username, password } => (1, username.clone(), password.clone(), String::new(), String::new(), String::new()),
            AuthConfig::SaslScram256 { username, password } => (2, username.clone(), password.clone(), String::new(), String::new(), String::new()),
            AuthConfig::SaslScram512 { username, password } => (3, username.clone(), password.clone(), String::new(), String::new(), String::new()),
            AuthConfig::Ssl { ca_cert, client_cert, client_key } => (
                4,
                String::new(),
                String::new(),
                ca_cert.clone().unwrap_or_default(),
                client_cert.clone().unwrap_or_default(),
                client_key.clone().unwrap_or_default(),
            ),
            AuthConfig::SaslSslPlain { username, password, ca_cert } => (5, username.clone(), password.clone(), ca_cert.clone().unwrap_or_default(), String::new(), String::new()),
            AuthConfig::SaslSslScram256 { username, password, ca_cert } => (6, username.clone(), password.clone(), ca_cert.clone().unwrap_or_default(), String::new(), String::new()),
            AuthConfig::SaslSslScram512 { username, password, ca_cert } => (7, username.clone(), password.clone(), ca_cert.clone().unwrap_or_default(), String::new(), String::new()),
        };
        Self {
            name: config.name.clone(),
            brokers: config.brokers.clone(),
            auth_type,
            username,
            password,
            ca_cert,
            client_cert,
            client_key,
            focused_field: 0,
            editing_index: Some(index),
        }
    }

    pub fn to_config(&self) -> ClusterConfig {
        use crate::config::AuthConfig;
        let ca = if self.ca_cert.is_empty() { None } else { Some(self.ca_cert.clone()) };
        let auth = match self.auth_type {
            1 => AuthConfig::SaslPlain {
                username: self.username.clone(),
                password: self.password.clone(),
            },
            2 => AuthConfig::SaslScram256 {
                username: self.username.clone(),
                password: self.password.clone(),
            },
            3 => AuthConfig::SaslScram512 {
                username: self.username.clone(),
                password: self.password.clone(),
            },
            4 => AuthConfig::Ssl {
                ca_cert: ca,
                client_cert: if self.client_cert.is_empty() { None } else { Some(self.client_cert.clone()) },
                client_key: if self.client_key.is_empty() { None } else { Some(self.client_key.clone()) },
            },
            5 => AuthConfig::SaslSslPlain {
                username: self.username.clone(),
                password: self.password.clone(),
                ca_cert: ca,
            },
            6 => AuthConfig::SaslSslScram256 {
                username: self.username.clone(),
                password: self.password.clone(),
                ca_cert: ca,
            },
            7 => AuthConfig::SaslSslScram512 {
                username: self.username.clone(),
                password: self.password.clone(),
                ca_cert: ca,
            },
            _ => AuthConfig::None,
        };
        ClusterConfig {
            name: self.name.clone(),
            brokers: self.brokers.clone(),
            auth,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionTestDialog {
    pub cluster_name: String,
    pub status: ConnectionTestStatus,
}

#[derive(Debug, Clone)]
pub enum ConnectionTestStatus {
    Testing,
    Success(String),
    Failed(String),
}

/// Sidebar navigation state
#[derive(Debug, Clone)]
pub struct SidebarState {
    pub items: Vec<SidebarItem>,
    pub selected: usize,
}

#[derive(Debug, Clone)]
pub struct SidebarItem {
    pub label: String,
    pub route: Route,
    pub indent: usize,
}

impl SidebarState {
    pub fn new() -> Self {
        Self {
            items: vec![
                SidebarItem { label: "Dashboard".to_string(), route: Route::Dashboard, indent: 0 },
                SidebarItem { label: "Topics".to_string(), route: Route::Topics, indent: 0 },
                SidebarItem { label: "Consumer Groups".to_string(), route: Route::ConsumerGroups, indent: 0 },
            ],
            selected: 0,
        }
    }

    pub fn next(&mut self) {
        if !self.items.is_empty() {
            self.selected = (self.selected + 1) % self.items.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.items.is_empty() {
            self.selected = (self.selected + self.items.len() - 1) % self.items.len();
        }
    }

    pub fn current_route(&self) -> Option<&Route> {
        self.items.get(self.selected).map(|item| &item.route)
    }
}

/// Dashboard state
#[derive(Debug, Clone, Default)]
pub struct DashboardState {
    pub controller_id: Option<i32>,
    pub broker_count: usize,
    pub brokers_online: Vec<i32>,
    pub topic_count: usize,
    pub partition_count: usize,
    pub loading: bool,
}

/// Topic list state
#[derive(Debug, Clone, Default)]
pub struct TopicState {
    pub topics: Vec<TopicInfo>,
    pub selected: usize,
    pub loading: bool,
    pub search_query: String,
}

impl TopicState {
    pub fn filtered_topics(&self) -> Vec<&TopicInfo> {
        if self.search_query.is_empty() {
            self.topics.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.topics.iter().filter(|t| t.name.to_lowercase().contains(&query)).collect()
        }
    }

    pub fn next(&mut self) {
        let len = self.filtered_topics().len();
        if len > 0 {
            self.selected = (self.selected + 1) % len;
        }
    }

    pub fn previous(&mut self) {
        let len = self.filtered_topics().len();
        if len > 0 {
            self.selected = (self.selected + len - 1) % len;
        }
    }
}

#[derive(Debug, Clone)]
pub struct TopicInfo {
    pub name: String,
    pub partitions: usize,
    pub replication_factor: usize,
    pub partition_details: Vec<PartitionInfo>,
}

#[derive(Debug, Clone)]
pub struct PartitionInfo {
    pub id: i32,
    pub leader: i32,
    pub replicas: Vec<i32>,
    pub isr: Vec<i32>,
}

/// Message browser state
#[derive(Debug, Clone)]
pub struct MessageState {
    pub topic: String,
    pub messages: Vec<KafkaMessage>,
    pub selected: usize,
    pub consuming: bool,
    pub offset_mode: OffsetMode,
    pub search_query: String,
    pub show_detail: bool,
    pub auto_scroll: bool,
}

impl Default for MessageState {
    fn default() -> Self {
        Self {
            topic: String::new(),
            messages: Vec::new(),
            selected: 0,
            consuming: false,
            offset_mode: OffsetMode::Latest,
            search_query: String::new(),
            show_detail: false,
            auto_scroll: true,
        }
    }
}

impl MessageState {
    pub fn new(topic: &str) -> Self {
        Self {
            topic: topic.to_string(),
            ..Default::default()
        }
    }

    pub fn filtered_messages(&self) -> Vec<&KafkaMessage> {
        if self.search_query.is_empty() {
            self.messages.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.messages
                .iter()
                .filter(|m| {
                    m.key.as_deref().unwrap_or("").to_lowercase().contains(&query)
                        || m.value.to_lowercase().contains(&query)
                })
                .collect()
        }
    }

    pub fn next(&mut self) {
        let len = self.filtered_messages().len();
        if len > 0 {
            self.selected = (self.selected + 1) % len;
        }
    }

    pub fn previous(&mut self) {
        let len = self.filtered_messages().len();
        if len > 0 {
            self.selected = (self.selected + len - 1) % len;
        }
    }
}

#[derive(Debug, Clone)]
pub enum OffsetMode {
    Earliest,
    Latest,
    Specific(i64),
    Timestamp(i64),
}

#[derive(Debug, Clone)]
pub struct KafkaMessage {
    pub partition: i32,
    pub offset: i64,
    pub key: Option<String>,
    pub value: String,
    pub timestamp: Option<i64>,
    pub headers: Vec<(String, String)>,
}

/// Consumer group state
#[derive(Debug, Clone, Default)]
pub struct ConsumerGroupState {
    pub groups: Vec<ConsumerGroupInfo>,
    pub selected: usize,
    pub loading: bool,
}

impl ConsumerGroupState {
    pub fn next(&mut self) {
        if !self.groups.is_empty() {
            self.selected = (self.selected + 1) % self.groups.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.groups.is_empty() {
            self.selected = (self.selected + self.groups.len() - 1) % self.groups.len();
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConsumerGroupInfo {
    pub name: String,
    pub state: String,
    pub members: usize,
    pub topics: Vec<String>,
    pub lag: Vec<PartitionLag>,
    pub total_lag: i64,
}

#[derive(Debug, Clone)]
pub struct PartitionLag {
    pub topic: String,
    pub partition: i32,
    pub current_offset: i64,
    pub log_end_offset: i64,
    pub lag: i64,
}

/// Topic detail state
#[derive(Debug, Clone, Default)]
pub struct TopicDetailState {
    pub topic_name: String,
    pub partitions: Vec<PartitionInfo>,
    pub config: Vec<(String, String)>,
    pub selected_partition: usize,
}

/// Log entry for status bar
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

/// Main application state
pub struct App {
    pub config: AppConfig,
    pub route: Route,
    pub active_cluster: Option<usize>,
    pub sidebar: SidebarState,
    pub dashboard: DashboardState,
    pub topics: TopicState,
    pub topic_detail: TopicDetailState,
    pub messages: MessageState,
    pub consumer_groups: ConsumerGroupState,
    pub dialog: Option<Dialog>,
    pub show_help: bool,
    pub focus: Focus,
    pub running: bool,
    pub status_message: String,
    pub logs: Vec<LogEntry>,
    pub cluster_select_index: usize,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            route: Route::ClusterSelect,
            active_cluster: None,
            sidebar: SidebarState::new(),
            dashboard: DashboardState::default(),
            topics: TopicState::default(),
            topic_detail: TopicDetailState::default(),
            messages: MessageState::default(),
            consumer_groups: ConsumerGroupState::default(),
            dialog: None,
            show_help: false,
            focus: Focus::Content,
            running: true,
            status_message: "Welcome to Kafka Eye".to_string(),
            logs: Vec::new(),
            cluster_select_index: 0,
        }
    }

    pub fn log(&mut self, level: LogLevel, message: &str) {
        self.logs.push(LogEntry {
            timestamp: chrono::Local::now(),
            level,
            message: message.to_string(),
        });
        // Keep only last 100 logs
        if self.logs.len() > 100 {
            self.logs.remove(0);
        }
    }

    pub fn log_info(&mut self, message: &str) {
        self.status_message = message.to_string();
        self.log(LogLevel::Info, message);
    }

    pub fn log_error(&mut self, message: &str) {
        self.status_message = format!("ERROR: {}", message);
        self.log(LogLevel::Error, message);
    }

    pub fn active_cluster_config(&self) -> Option<&ClusterConfig> {
        self.active_cluster.and_then(|i| self.config.clusters.get(i))
    }

    pub fn navigate(&mut self, route: Route) {
        self.route = route;
        self.focus = Focus::Content;
    }
}
