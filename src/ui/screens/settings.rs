use anyhow::Result;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::{
    app::state::{AppState, AppMode},
    config::Config,
};

pub fn render(f: &mut Frame, area: Rect, state: &AppState, config: &Config) -> Result<()> {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(2),  // Spacer
            Constraint::Length(3),  // Mode info
            Constraint::Length(2),  // Spacer
            Constraint::Length(10), // Cluster info
            Constraint::Min(0),     // Command area
        ])
        .split(area);

    // Title
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title("Settings")
            .title_style(Style::default().fg(Color::Green)),
        chunks[0],
    );

    // Mode info
    let mode_text = match state.mode {
        AppMode::Normal => "Normal Mode - Navigate with hjkl, q to quit",
        AppMode::Insert => "Insert Mode - Enter text, ESC to return to normal mode",
        AppMode::Command => "Command Mode - Enter command, ESC to cancel",
        AppMode::Visual => "Visual Mode - Select with hjkl, ESC to cancel",
    };
    f.render_widget(
        Paragraph::new(mode_text)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Mode")),
        chunks[2],
    );

    // Cluster info
    let mut cluster_text = String::new();
    cluster_text.push_str("Configured clusters:\n");
    for name in config.list_clusters() {
        let active = config.get_active_cluster().map_or(false, |(active, _)| active == name);
        if active {
            cluster_text.push_str(&format!("* {} (active)\n", name));
        } else {
            cluster_text.push_str(&format!("  {}\n", name));
        }
    }
    cluster_text.push_str("\nCommands:\n");
    cluster_text.push_str("cluster add <name> <broker1,broker2,...> - Add a new cluster\n");
    cluster_text.push_str("cluster remove|rm <name> - Remove a cluster\n");
    cluster_text.push_str("cluster switch|use <name> - Switch to a different cluster\n");
    cluster_text.push_str("cluster list|ls - List all configured clusters\n");

    f.render_widget(
        Paragraph::new(cluster_text)
            .style(Style::default())
            .block(Block::default().borders(Borders::ALL).title("Cluster Management")),
        chunks[4],
    );

    // Command area (if in command mode)
    if state.mode == AppMode::Command {
        f.render_widget(
            Paragraph::new(Text::raw(&state.command_input))
                .style(Style::default())
                .block(Block::default().borders(Borders::ALL).title("Command")),
            chunks[5],
        );
    }

    Ok(())
}
