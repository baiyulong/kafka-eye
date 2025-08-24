use anyhow::Result;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

use crate::app::state::AppState;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) -> Result<()> {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // Stats overview
            Constraint::Min(0),     // Recent activity
        ])
        .split(area);

    render_stats_overview(f, chunks[0], state)?;
    render_recent_activity(f, chunks[1], state)?;

    Ok(())
}

fn render_stats_overview(f: &mut Frame, area: Rect, state: &AppState) -> Result<()> {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    // Topics count
    let topics_block = Block::default()
        .title("Topics")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White));

    let topics_content = Paragraph::new(format!("{}", state.stats.total_topics))
        .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .block(topics_block);

    f.render_widget(topics_content, chunks[0]);

    // Partitions count
    let partitions_block = Block::default()
        .title("Partitions")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White));

    let partitions_content = Paragraph::new(format!("{}", state.stats.total_partitions))
        .style(Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD))
        .block(partitions_block);

    f.render_widget(partitions_content, chunks[1]);

    // Consumer Groups count
    let groups_block = Block::default()
        .title("Consumer Groups")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White));

    let groups_content = Paragraph::new(format!("{}", state.stats.total_consumer_groups))
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .block(groups_block);

    f.render_widget(groups_content, chunks[2]);

    // Throughput
    let throughput_block = Block::default()
        .title("Messages/sec")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White));

    let throughput_content = Paragraph::new(format!("{:.2}", state.stats.messages_per_sec))
        .style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
        .block(throughput_block);

    f.render_widget(throughput_content, chunks[3]);

    Ok(())
}

fn render_recent_activity(f: &mut Frame, area: Rect, state: &AppState) -> Result<()> {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Recent topics
            Constraint::Percentage(50), // Recent messages
        ])
        .split(area);

    // Recent topics
    let recent_topics: Vec<ListItem> = state
        .topics
        .iter()
        .take(10)
        .map(|topic| {
            ListItem::new(Line::from(vec![
                Span::styled(&topic.name, Style::default().fg(Color::White)),
                Span::styled(
                    format!(" ({}p, {}r)", topic.partitions, topic.replicas),
                    Style::default().fg(Color::Gray),
                ),
            ]))
        })
        .collect();

    let topics_list = List::new(recent_topics)
        .block(Block::default().title("Recent Topics").borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow));

    f.render_widget(topics_list, chunks[0]);

    // Recent messages
    let recent_messages: Vec<ListItem> = state
        .messages
        .iter()
        .rev()
        .take(10)
        .map(|message| {
            let timestamp = message.timestamp.format("%H:%M:%S").to_string();
            ListItem::new(Line::from(vec![
                Span::styled(timestamp, Style::default().fg(Color::Gray)),
                Span::styled(" ", Style::default()),
                Span::styled(&message.topic, Style::default().fg(Color::Cyan)),
                Span::styled(": ", Style::default()),
                Span::styled(
                    if message.value.len() > 50 {
                        format!("{}...", &message.value[..50])
                    } else {
                        message.value.clone()
                    },
                    Style::default().fg(Color::White),
                ),
            ]))
        })
        .collect();

    let messages_list = List::new(recent_messages)
        .block(Block::default().title("Recent Messages").borders(Borders::ALL))
        .style(Style::default().fg(Color::White));

    f.render_widget(messages_list, chunks[1]);

    Ok(())
}
