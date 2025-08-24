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
        let mut kafka_manager = KafkaManager::new(&config).await?;
        
        // Try to connect to Kafka if there's an active cluster
        if let Some((cluster_name, kafka_config)) = config.get_active_cluster() {
            if let Err(e) = kafka_manager.connect(kafka_config).await {
                warn!("Failed to connect to Kafka cluster {}: {}", cluster_name, e);
                // Don't fail here, just log the warning and continue
                state.set_connected(false, Some(cluster_name.to_string()));
            } else {
                state.set_connected(true, Some(cluster_name.to_string()));
            }
        } else {
            state.set_connected(false, None);
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
                    AppMode::Normal => self.handle_normal_mode_key(key).await?,
                    AppMode::Insert => self.handle_insert_mode_key(key).await?,
                    AppMode::Command => self.handle_command_mode_key(key).await?,
                    AppMode::Visual => self.handle_visual_mode_key(key).await?,
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
                // Try to connect to the new cluster if this is the first one
                if self.state.current_cluster.is_none() {
                    self.reconnect().await?;
                }
            }
            Command::RemoveCluster { ref name } => {
                self.state.handle_command(cmd.clone(), &mut self.config)?;
                // If we removed the current cluster, try to connect to another one
                if self.state.current_cluster.is_none() {
                    self.reconnect().await?;
                }
            }
            Command::SwitchCluster { ref name } => {
                self.state.handle_command(cmd.clone(), &mut self.config)?;
                // Try to connect to the new cluster
                self.reconnect().await?;
            }
            Command::ListClusters => {
                self.state.handle_command(cmd.clone(), &mut self.config)?;
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

    async fn reconnect(&mut self) -> Result<()> {
        if let Some((name, config)) = self.config.get_active_cluster() {
            info!("Connecting to cluster {}", name);
            // TODO: Update the KafkaManager with the new config
            self.kafka_manager.disconnect().await?;
            self.kafka_manager.connect(config).await?;
            self.state.set_connected(true, Some(name.to_string()));
        } else {
            info!("No active cluster to connect to");
            self.state.set_connected(false, None);
        }
        Ok(())
    }

    async fn connect_to_cluster(&mut self, _cluster: &str) -> Result<()> {
        info!("Connecting to cluster...");
        // TODO: Implement cluster connection
        Ok(())
    }
}
