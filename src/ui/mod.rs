pub mod components;
pub mod screens;

use anyhow::Result;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs},
    Frame,
};

use crate::app::state::{AppMode, AppState, Screen};

pub struct UI {
    // UI state if needed
}

impl UI {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render<B: Backend>(&self, f: &mut Frame<B>, state: &AppState) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Top tabs
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Status bar
            ])
            .split(f.area());

        // Render top tabs
        self.render_tabs(f, chunks[0], state);

        // Render main content based on current screen
        match state.current_screen {
            Screen::Dashboard => screens::dashboard::render(f, chunks[1], state)?,
            Screen::TopicList => screens::topics::render_topic_list(f, chunks[1], state)?,
            Screen::TopicDetail => screens::topics::render_topic_detail(f, chunks[1], state)?,
            Screen::MessageProducer => screens::messages::render_producer(f, chunks[1], state)?,
            Screen::MessageConsumer => screens::messages::render_consumer(f, chunks[1], state)?,
            Screen::ConsumerGroups => screens::consumer_groups::render(f, chunks[1], state)?,
            Screen::Monitoring => screens::monitoring::render(f, chunks[1], state)?,
            Screen::Settings => screens::settings::render(f, chunks[1], state)?,
        }

        // Render status bar
        self.render_status_bar(f, chunks[2], state);

        // Render command input if in command mode
        if state.mode == AppMode::Command {
            self.render_command_input(f, f.area(), state);
        }

        Ok(())
    }

    fn render_tabs<B: Backend>(&self, f: &mut Frame<B>, area: Rect, state: &AppState) {
        let titles = vec![
            "Dashboard",
            "Topics",
            "Producer",
            "Consumer", 
            "Groups",
            "Monitor",
            "Settings",
        ];

        let selected_index = match state.current_screen {
            Screen::Dashboard => 0,
            Screen::TopicList | Screen::TopicDetail => 1,
            Screen::MessageProducer => 2,
            Screen::MessageConsumer => 3,
            Screen::ConsumerGroups => 4,
            Screen::Monitoring => 5,
            Screen::Settings => 6,
        };

        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("Kafka Eye"))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .select(selected_index);

        f.render_widget(tabs, area);
    }

    fn render_status_bar<B: Backend>(&self, f: &mut Frame<B>, area: Rect, state: &AppState) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),    // Left side
                Constraint::Length(20), // Right side
            ])
            .split(area);

        // Left side - connection status and current info
        let left_content = format!(
            " {} | Mode: {} | {}",
            state.connection_status,
            match state.mode {
                AppMode::Normal => "NORMAL",
                AppMode::Insert => "INSERT",
                AppMode::Command => "COMMAND",
                AppMode::Visual => "VISUAL",
            },
            state.selected_topic.as_deref().unwrap_or("No topic selected")
        );

        let left_paragraph = Paragraph::new(left_content)
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(left_paragraph, chunks[0]);

        // Right side - help text
        let help_text = match state.mode {
            AppMode::Normal => "q:quit :cmd Tab:nav",
            AppMode::Insert => "ESC:normal Enter:send",
            AppMode::Command => "ESC:cancel Enter:exec",
            AppMode::Visual => "ESC:normal",
        };

        let right_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(right_paragraph, chunks[1]);
    }

    fn render_command_input<B: Backend>(&self, f: &mut Frame<B>, area: Rect, state: &AppState) {
        let popup_area = self.centered_rect(60, 3, area);

        // Clear the area
        f.render_widget(Clear, popup_area);

        let input_text = format!(":{}", state.command_input);
        let input_paragraph = Paragraph::new(input_text)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Command"));

        f.render_widget(input_paragraph, popup_area);
    }

    fn centered_rect(&self, percent_x: u16, height: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - height) / 2),
                Constraint::Length(height),
                Constraint::Percentage((100 - height) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}

impl Default for UI {
    fn default() -> Self {
        Self::new()
    }
}
