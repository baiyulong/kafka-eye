pub mod commands;
pub mod events;
pub mod state;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, EnableMouseCapture, DisableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::kafka::KafkaManager;
use crate::ui::UI;
use commands::Command;
use events::{AppEvent, InputEvent};
use state::{AppMode, AppState, Screen};
use std::time::Instant;
use std::io;

pub struct App {
    state: AppState,
    ui: UI,
    kafka_manager: KafkaManager,
    config: Config,
    event_rx: mpsc::UnboundedReceiver<AppEvent>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    should_quit: bool,
}

impl App {
    pub async fn new(config: Config) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        let mut state = AppState::new();
        let ui = UI::new();
        let kafka_manager = KafkaManager::new(&config).await?;
        
        // Initialize state without auto-connecting
        // Wait for user to manually select and connect to a cluster
        if let Some((cluster_name, _)) = config.get_active_cluster() {
            state.set_connected(false, Some(cluster_name.to_string()));
            state.connection_status = format!("Ready to connect to cluster: {}. Use ':connect' to connect or ':status' for more info.", cluster_name);
        } else {
            state.set_connected(false, None);
            state.connection_status = "No cluster configured. Use ':cluster add <name> <brokers>' to add a cluster or ':status' for help.".to_string();
        }

        Ok(Self {
            state,
            ui,
            kafka_manager,
            config,
            event_rx,
            event_tx,
            should_quit: false,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Clone event sender for input handler
        let event_tx = self.event_tx.clone();
        
        // Spawn input handler task
        tokio::spawn(async move {
            let mut last_tick = Instant::now();
            let tick_rate = Duration::from_millis(250);

            loop {
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or_else(|| Duration::from_secs(0));

                if crossterm::event::poll(timeout).unwrap_or(false) {
                    if let Ok(event) = event::read() {
                        match event {
                            Event::Key(key) => {
                                if key.kind == KeyEventKind::Press {
                                    if event_tx.send(AppEvent::Input(InputEvent::Key(key))).is_err() {
                                        break;
                                    }
                                }
                            }
                            Event::Mouse(mouse) => {
                                if event_tx.send(AppEvent::Input(InputEvent::Mouse(mouse))).is_err() {
                                    break;
                                }
                            }
                            Event::Resize(w, h) => {
                                if event_tx.send(AppEvent::Input(InputEvent::Resize(w, h))).is_err() {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                }

                if last_tick.elapsed() >= tick_rate {
                    if event_tx.send(AppEvent::Tick).is_err() {
                        break;
                    }
                    last_tick = Instant::now();
                }
            }
        });

        // Main application loop
        while !self.should_quit {
            // Draw UI
            terminal.draw(|f| {
                if let Err(e) = self.ui.render(f, &self.state, &self.config) {
                    error!("Failed to render UI: {}", e);
                }
            })?;

            // Handle events
            if let Ok(event) = self.event_rx.try_recv() {
                self.handle_event(event).await?;
            }

            // Small delay to prevent busy waiting
            tokio::time::sleep(Duration::from_millis(16)).await;
        }

        // Cleanup Kafka connection
        if let Err(e) = self.kafka_manager.disconnect().await {
            error!("Failed to disconnect from Kafka: {}", e);
        }

        // Cleanup terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        Ok(())
    }

    async fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        debug!("Handling event: {:?}", event);

        match event {
            AppEvent::Input(input_event) => {
                self.handle_input_event(input_event).await?;
            }
            AppEvent::Tick => {
                // Handle periodic updates
                self.handle_tick().await?;
            }
            AppEvent::KafkaEvent(kafka_event) => {
                // Handle Kafka-related events
                self.handle_kafka_event(kafka_event).await?;
            }
        }

        Ok(())
    }

    async fn handle_input_event(&mut self, input_event: InputEvent) -> Result<()> {
        match input_event {
            InputEvent::Key(key) => {
                match self.state.mode {
                    AppMode::Normal => {
                        match self.state.current_screen {
                            Screen::ClusterManagement => self.handle_cluster_management_key(key).await?,
                            _ => self.handle_normal_mode_key(key).await?,
                        }
                    }
                    AppMode::Insert => self.handle_insert_mode_key(key).await?,
                    AppMode::Command => self.handle_command_mode_key(key).await?,
                    AppMode::Visual => self.handle_visual_mode_key(key).await?,
                    AppMode::ClusterForm => self.handle_cluster_form_key(key).await?,
                }
            }
            InputEvent::Mouse(_mouse) => {
                // Handle mouse events if needed
            }
            InputEvent::Resize(w, h) => {
                info!("Terminal resized to {}x{}", w, h);
            }
        }

        Ok(())
    }

    async fn handle_normal_mode_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char(':') => {
                self.state.mode = AppMode::Command;
                self.state.command_input.clear();
            }
            KeyCode::Char('i') => {
                self.state.mode = AppMode::Insert;
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.state.move_left();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.state.move_down();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.state.move_up();
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.state.move_right();
            }
            KeyCode::Char('g') => {
                // Handle 'gg' for go to top
                if self.state.last_key == Some('g') {
                    self.state.go_to_top();
                }
                self.state.last_key = Some('g');
            }
            KeyCode::Char('G') => {
                self.state.go_to_bottom();
            }
            KeyCode::Char('r') => {
                self.refresh_current_screen().await?;
            }
            KeyCode::Tab => {
                self.state.next_screen();
            }
            KeyCode::BackTab => {
                self.state.previous_screen();
            }
            _ => {
                self.state.last_key = None;
            }
        }

        Ok(())
    }

    async fn handle_insert_mode_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.state.mode = AppMode::Normal;
            }
            KeyCode::Char(c) => {
                self.state.input_buffer.push(c);
            }
            KeyCode::Backspace => {
                self.state.input_buffer.pop();
            }
            KeyCode::Enter => {
                // Handle message send or other actions
                self.handle_insert_enter().await?;
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_command_mode_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.state.mode = AppMode::Normal;
                self.state.command_input.clear();
            }
            KeyCode::Char(c) => {
                self.state.command_input.push(c);
            }
            KeyCode::Backspace => {
                self.state.command_input.pop();
            }
            KeyCode::Enter => {
                let command = self.state.command_input.clone();
                self.state.command_input.clear();
                self.state.mode = AppMode::Normal;
                self.execute_command(command).await?;
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_visual_mode_key(&mut self, _key: crossterm::event::KeyEvent) -> Result<()> {
        // TODO: Implement visual mode handling
        Ok(())
    }

    async fn handle_tick(&mut self) -> Result<()> {
        // Handle periodic updates like refreshing data
        Ok(())
    }

    async fn handle_kafka_event(&mut self, _kafka_event: crate::kafka::KafkaEvent) -> Result<()> {
        // Handle Kafka-specific events
        Ok(())
    }

    async fn handle_insert_enter(&mut self) -> Result<()> {
        match self.state.current_screen {
            Screen::MessageProducer => {
                // Send message to Kafka
                if !self.state.input_buffer.is_empty() {
                    // TODO: Implement message sending
                    info!("Sending message: {}", self.state.input_buffer);
                    self.state.input_buffer.clear();
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn execute_command(&mut self, command: String) -> Result<()> {
        let cmd = Command::parse(&command);
        match cmd {
            Command::AddCluster { ref name, ref brokers, ref client_id, ref security } => {
                self.state.handle_command(cmd.clone(), &mut self.config)?;
                // Don't auto-connect, just update the status
                if let Some((cluster_name, _)) = self.config.get_active_cluster() {
                    self.state.connection_status = format!("Cluster '{}' added. Use 'connect' to connect.", cluster_name);
                }
            }
            Command::RemoveCluster { ref name } => {
                self.state.handle_command(cmd.clone(), &mut self.config)?;
                // Don't auto-connect, just update the status
                if let Some((cluster_name, _)) = self.config.get_active_cluster() {
                    self.state.connection_status = format!("Ready to connect to cluster: {}", cluster_name);
                } else {
                    self.state.connection_status = "No cluster configured. Use 'cluster add' to add a cluster.".to_string();
                }
            }
            Command::SwitchCluster { ref name } => {
                self.state.handle_command(cmd.clone(), &mut self.config)?;
                // Don't auto-connect when switching clusters, wait for user to use 'connect'
                if let Some((cluster_name, _)) = self.config.get_active_cluster() {
                    self.state.set_connected(false, Some(cluster_name.to_string()));
                    self.state.connection_status = format!("Switched to cluster '{}'. Use 'connect' to connect.", cluster_name);
                }
            }
            Command::ListClusters => {
                self.state.handle_command(cmd.clone(), &mut self.config)?;
            }
            Command::ManageClusters => {
                self.open_cluster_management();
            }
            Command::Status => {
                self.show_status();
            }
            Command::Connect => {
                if let Some((cluster_name, kafka_config)) = self.config.get_active_cluster() {
                    match self.kafka_manager.connect(kafka_config).await {
                        Ok(()) => {
                            self.state.set_connected(true, Some(cluster_name.to_string()));
                            info!("Connected to cluster: {}", cluster_name);
                        }
                        Err(e) => {
                            self.state.set_connected(false, Some(cluster_name.to_string()));
                            self.state.connection_status = format!("Failed to connect: {}", e);
                            error!("Failed to connect to cluster {}: {}", cluster_name, e);
                        }
                    }
                } else {
                    self.state.set_connected(false, None);
                    self.state.connection_status = "No active cluster configured".to_string();
                    warn!("No active cluster configured");
                }
            }
            Command::Disconnect => {
                match self.kafka_manager.disconnect().await {
                    Ok(()) => {
                        self.state.set_connected(false, self.state.current_cluster.clone());
                        info!("Disconnected from Kafka cluster");
                    }
                    Err(e) => {
                        self.state.connection_status = format!("Failed to disconnect: {}", e);
                        error!("Failed to disconnect: {}", e);
                    }
                }
            }
            Command::Quit => {
                self.should_quit = true;
            }
            Command::Unknown(msg) => {
                warn!("Unknown command: {}", msg);
            }
        }
        Ok(())
    }

    fn show_status(&mut self) {
        let mut status_info = vec![];
        
        // Connection status
        if self.state.connected {
            status_info.push(format!("✓ Connected to cluster: {}", 
                self.state.current_cluster.as_deref().unwrap_or("Unknown")));
        } else {
            status_info.push("✗ Not connected to any cluster".to_string());
        }
        
        // Active cluster
        if let Some((cluster_name, _)) = self.config.get_active_cluster() {
            status_info.push(format!("Active cluster: {}", cluster_name));
        } else {
            status_info.push("No active cluster configured".to_string());
        }
        
        // Available clusters
        let cluster_names: Vec<String> = self.config.clusters.keys().cloned().collect();
        if !cluster_names.is_empty() {
            status_info.push(format!("Available clusters: {}", cluster_names.join(", ")));
        } else {
            status_info.push("No clusters configured".to_string());
        }
        
        // Next steps
        status_info.push("".to_string()); // Empty line
        if !self.state.connected {
            if self.config.get_active_cluster().is_some() {
                status_info.push("Next steps: Use 'connect' to connect to the active cluster".to_string());
            } else if !cluster_names.is_empty() {
                status_info.push("Next steps: Use 'cluster switch <name>' to select a cluster, then 'connect'".to_string());
            } else {
                status_info.push("Next steps: Use 'cluster add <name> <brokers>' to add a cluster".to_string());
            }
        }
        
        self.state.connection_status = status_info.join(" | ");
    }

    fn open_cluster_management(&mut self) {
        // Load cluster list
        self.state.cluster_list = self.config.clusters.keys().cloned().collect();
        self.state.cluster_list.sort();
        
        // Start with cluster selection screen
        self.state.current_screen = Screen::ClusterManagement;
        self.state.mode = AppMode::Normal;
        self.state.selected_index = 0;
    }

    async fn handle_cluster_form_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;
        
        match key.code {
            KeyCode::Esc => {
                self.state.exit_cluster_form();
            }
            KeyCode::Tab => {
                self.state.cluster_form_next_field();
            }
            KeyCode::BackTab => {
                self.state.cluster_form_prev_field();
            }
            KeyCode::Enter => {
                if self.state.cluster_form.current_field == 7 || 
                   (self.state.cluster_form_action == crate::app::state::ClusterFormAction::Delete) {
                    // Submit form
                    self.submit_cluster_form().await?;
                } else {
                    self.state.cluster_form_next_field();
                }
            }
            KeyCode::Backspace => {
                self.state.cluster_form_backspace();
            }
            KeyCode::Char(c) => {
                self.state.cluster_form_add_char(c);
            }
            _ => {}
        }
        Ok(())
    }

    async fn submit_cluster_form(&mut self) -> Result<()> {
        use crate::app::state::ClusterFormAction;
        
        match self.state.cluster_form_action {
            ClusterFormAction::Add => {
                let form = &self.state.cluster_form;
                if form.name.is_empty() || form.brokers.is_empty() {
                    self.state.connection_status = "Cluster name and brokers are required".to_string();
                    return Ok(());
                }
                
                let brokers: Vec<String> = form.brokers.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                
                self.config.add_cluster(
                    &form.name,
                    &brokers,
                    &form.client_id,
                    None // TODO: Add security config
                )?;
                
                self.config.save("config.yaml")?;
                self.state.connection_status = format!("Cluster '{}' added successfully", form.name);
            }
            ClusterFormAction::Edit => {
                // TODO: Implement edit functionality
                self.state.connection_status = "Edit functionality not yet implemented".to_string();
            }
            ClusterFormAction::Delete => {
                let cluster_name = &self.state.cluster_form.name;
                self.config.remove_cluster(cluster_name)?;
                self.config.save("config.yaml")?;
                self.state.connection_status = format!("Cluster '{}' deleted successfully", cluster_name);
            }
        }
        
        self.state.exit_cluster_form();
        Ok(())
    }

    async fn handle_cluster_management_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;
        
        match key.code {
            KeyCode::Esc => {
                self.state.current_screen = Screen::Dashboard;
                self.state.mode = AppMode::Normal;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.state.selected_index > 0 {
                    self.state.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.state.selected_index < self.state.cluster_list.len() {
                    self.state.selected_index += 1;
                }
            }
            KeyCode::Char('a') => {
                // Add new cluster
                self.state.start_cluster_form(crate::app::state::ClusterFormAction::Add, None);
            }
            KeyCode::Char('e') | KeyCode::Enter => {
                // Edit selected cluster
                if self.state.selected_index < self.state.cluster_list.len() {
                    let cluster_name = self.state.cluster_list[self.state.selected_index].clone();
                    self.state.start_cluster_form(crate::app::state::ClusterFormAction::Edit, Some(cluster_name));
                }
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                // Delete selected cluster
                if self.state.selected_index < self.state.cluster_list.len() {
                    let cluster_name = self.state.cluster_list[self.state.selected_index].clone();
                    self.state.start_cluster_form(crate::app::state::ClusterFormAction::Delete, Some(cluster_name));
                }
            }
            KeyCode::Char('s') => {
                // Switch to selected cluster
                if self.state.selected_index < self.state.cluster_list.len() {
                    let cluster_name = self.state.cluster_list[self.state.selected_index].clone();
                    self.config.set_active_cluster(&cluster_name)?;
                    self.config.save("config.yaml")?;
                    self.state.set_connected(false, Some(cluster_name.clone()));
                    self.state.connection_status = format!("Switched to cluster '{}'. Use 'connect' to connect.", cluster_name);
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn refresh_current_screen(&mut self) -> Result<()> {
        match self.state.current_screen {
            Screen::TopicList => self.refresh_topics().await?,
            Screen::ConsumerGroups => self.refresh_consumer_groups().await?,
            Screen::Dashboard => self.refresh_dashboard().await?,
            _ => {}
        }

        Ok(())
    }

    async fn refresh_topics(&mut self) -> Result<()> {
        info!("Refreshing topics...");
        // TODO: Implement topic refresh
        Ok(())
    }

    async fn refresh_consumer_groups(&mut self) -> Result<()> {
        info!("Refreshing consumer groups...");
        // TODO: Implement consumer group refresh
        Ok(())
    }

    async fn refresh_dashboard(&mut self) -> Result<()> {
        info!("Refreshing dashboard...");
        // TODO: Implement dashboard refresh
        Ok(())
    }

    async fn connect_to_active_cluster(&mut self) -> Result<()> {
        if let Some((name, config)) = self.config.get_active_cluster() {
            info!("Connecting to cluster {}", name);
            self.kafka_manager.disconnect().await?;
            self.kafka_manager.connect(config).await?;
            self.state.set_connected(true, Some(name.to_string()));
        } else {
            info!("No active cluster to connect to");
            self.state.set_connected(false, None);
            self.state.connection_status = "No active cluster configured".to_string();
        }
        Ok(())
    }

    async fn connect_to_cluster(&mut self, _cluster: &str) -> Result<()> {
        info!("Connecting to cluster...");
        // TODO: Implement cluster connection
        Ok(())
    }
}
