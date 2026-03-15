use crate::app::*;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_dashboard(app: &App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(7),
            Constraint::Min(5),
        ])
        .split(area);

    // Title
    let title = Paragraph::new(" Cluster Dashboard")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(title, chunks[0]);

    // Stats cards
    let stats_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(chunks[1]);

    let ds = &app.dashboard;

    let controller_text = match ds.controller_id {
        Some(id) => format!("Controller: Broker {}", id),
        None => "Controller: Unknown".to_string(),
    };
    render_stat_card(frame, stats_chunks[0], "Controller", &controller_text, Color::Cyan);

    let broker_status = format!("{} / {} online", ds.brokers_online.len(), ds.broker_count);
    let broker_color = if ds.brokers_online.len() == ds.broker_count && ds.broker_count > 0 {
        Color::Green
    } else if ds.brokers_online.is_empty() {
        Color::Red
    } else {
        Color::Yellow
    };
    render_stat_card(frame, stats_chunks[1], "Brokers", &broker_status, broker_color);
    render_stat_card(frame, stats_chunks[2], "Topics", &ds.topic_count.to_string(), Color::Cyan);
    render_stat_card(frame, stats_chunks[3], "Partitions", &ds.partition_count.to_string(), Color::Cyan);

    // Broker list
    let broker_ids: Vec<String> = ds.brokers_online.iter().map(|b| format!("Broker {} ● Online", b)).collect();
    let broker_text = if broker_ids.is_empty() {
        "No broker data. Press 'r' to refresh.".to_string()
    } else {
        broker_ids.join("\n")
    };
    let broker_info = Paragraph::new(broker_text)
        .style(Style::default().fg(Color::Green))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Broker Details ")
                .title_style(Style::default().fg(Color::Cyan))
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    frame.render_widget(broker_info, chunks[2]);
}

fn render_stat_card(frame: &mut Frame, area: Rect, title: &str, value: &str, color: Color) {
    let card = Paragraph::new(vec![
        Line::from(Span::styled(title, Style::default().fg(Color::DarkGray))),
        Line::from(""),
        Line::from(Span::styled(value, Style::default().fg(color).add_modifier(Modifier::BOLD))),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(card, area);
}
