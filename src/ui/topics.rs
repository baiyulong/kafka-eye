use crate::app::*;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_topics(app: &App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(area);

    // Title
    let title = Paragraph::new(" Topics")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(title, chunks[0]);

    // Search bar
    let search_style = if app.focus == Focus::Search {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let search = Paragraph::new(format!(" 🔍 {}", if app.topics.search_query.is_empty() { "Type to filter..." } else { &app.topics.search_query }))
        .style(search_style)
        .block(Block::default().borders(Borders::ALL).title(" Search ").border_style(search_style));
    frame.render_widget(search, chunks[1]);

    // Topic table
    let filtered = app.topics.filtered_topics();
    let header = Row::new(vec![
        Cell::from("Topic Name").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Partitions").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Repl. Factor").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]).height(1);

    let rows: Vec<Row> = filtered.iter().enumerate().map(|(i, topic)| {
        let style = if i == app.topics.selected {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };
        Row::new(vec![
            Cell::from(topic.name.clone()),
            Cell::from(topic.partitions.to_string()),
            Cell::from(topic.replication_factor.to_string()),
        ]).style(style)
    }).collect();

    let table = Table::new(rows, [
        Constraint::Min(30),
        Constraint::Length(12),
        Constraint::Length(14),
    ])
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Topics ({}) ", filtered.len()))
            .title_style(Style::default().fg(Color::Cyan))
            .border_style(Style::default().fg(if app.focus == Focus::Content { Color::Cyan } else { Color::DarkGray })),
    );
    frame.render_widget(table, chunks[2]);

    // Help line
    let help = Paragraph::new(" c: Create | d: Delete | Enter: Detail | m: Messages | /: Search | r: Refresh ")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[3]);
}

pub fn render_topic_detail(app: &App, frame: &mut Frame, area: Rect, topic_name: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(area);

    let title = Paragraph::new(format!(" Topic: {}", topic_name))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(title, chunks[0]);

    // Partition details
    let header = Row::new(vec![
        Cell::from("Partition").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Leader").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Replicas").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("ISR").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Status").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]).height(1);

    let rows: Vec<Row> = app.topic_detail.partitions.iter().enumerate().map(|(i, p)| {
        let status = if p.isr.len() == p.replicas.len() {
            Span::styled("● Synced", Style::default().fg(Color::Green))
        } else {
            Span::styled("● Under-replicated", Style::default().fg(Color::Yellow))
        };
        let style = if i == app.topic_detail.selected_partition {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };
        Row::new(vec![
            Cell::from(p.id.to_string()),
            Cell::from(format!("Broker {}", p.leader)),
            Cell::from(format!("{:?}", p.replicas)),
            Cell::from(format!("{:?}", p.isr)),
            Cell::from(status),
        ]).style(style)
    }).collect();

    let table = Table::new(rows, [
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Min(15),
        Constraint::Min(15),
        Constraint::Length(20),
    ])
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Partitions ({}) ", app.topic_detail.partitions.len()))
            .title_style(Style::default().fg(Color::Cyan))
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(table, chunks[1]);

    let help = Paragraph::new(" Esc: Back | m: Messages | r: Refresh ")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
}
