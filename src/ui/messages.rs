use crate::app::*;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_messages(app: &App, frame: &mut Frame, area: Rect, topic: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(area);

    // Title with consuming status
    let status_icon = if app.messages.consuming { "⏺ Consuming" } else { "⏸ Paused" };
    let status_color = if app.messages.consuming { Color::Green } else { Color::Yellow };
    let title = Line::from(vec![
        Span::styled(format!(" Messages: {} ", topic), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(format!("[{}]", status_icon), Style::default().fg(status_color)),
        Span::styled(format!(" ({} msgs)", app.messages.messages.len()), Style::default().fg(Color::DarkGray)),
    ]);
    let title_widget = Paragraph::new(title)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(title_widget, chunks[0]);

    // Search/filter bar
    let search_style = if app.focus == Focus::Search {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let search = Paragraph::new(format!(" 🔍 {}", if app.messages.search_query.is_empty() { "Filter messages..." } else { &app.messages.search_query }))
        .style(search_style)
        .block(Block::default().borders(Borders::ALL).title(" Filter ").border_style(search_style));
    frame.render_widget(search, chunks[1]);

    if app.messages.show_detail {
        // Detail view of selected message
        let filtered = app.messages.filtered_messages();
        if let Some(msg) = filtered.get(app.messages.selected) {
            let mut text = Vec::new();
            text.push(Line::from(vec![
                Span::styled("Partition: ", Style::default().fg(Color::DarkGray)),
                Span::styled(msg.partition.to_string(), Style::default().fg(Color::White)),
                Span::styled("  Offset: ", Style::default().fg(Color::DarkGray)),
                Span::styled(msg.offset.to_string(), Style::default().fg(Color::White)),
            ]));
            if let Some(ts) = msg.timestamp {
                let dt = chrono::DateTime::from_timestamp_millis(ts)
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S%.3f").to_string())
                    .unwrap_or_else(|| ts.to_string());
                text.push(Line::from(vec![
                    Span::styled("Timestamp: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(dt, Style::default().fg(Color::White)),
                ]));
            }
            text.push(Line::from(vec![
                Span::styled("Key: ", Style::default().fg(Color::DarkGray)),
                Span::styled(msg.key.as_deref().unwrap_or("<null>"), Style::default().fg(Color::Yellow)),
            ]));
            if !msg.headers.is_empty() {
                text.push(Line::from(Span::styled("Headers:", Style::default().fg(Color::DarkGray))));
                for (k, v) in &msg.headers {
                    text.push(Line::from(format!("  {}: {}", k, v)));
                }
            }
            text.push(Line::from(Span::styled("Value:", Style::default().fg(Color::DarkGray))));

            // Try to pretty-print JSON
            let value_display = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&msg.value) {
                serde_json::to_string_pretty(&json).unwrap_or_else(|_| msg.value.clone())
            } else {
                msg.value.clone()
            };
            for line in value_display.lines() {
                text.push(Line::from(Span::styled(format!("  {}", line), Style::default().fg(Color::Green))));
            }

            let detail = Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Message Detail ")
                        .title_style(Style::default().fg(Color::Cyan))
                        .border_style(Style::default().fg(Color::Cyan)),
                );
            frame.render_widget(detail, chunks[2]);
        }
    } else {
        // Table view
        let filtered = app.messages.filtered_messages();
        let header = Row::new(vec![
            Cell::from("Part").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Offset").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Key").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Value").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Timestamp").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]).height(1);

        let rows: Vec<Row> = filtered.iter().enumerate().map(|(i, msg)| {
            let ts = msg.timestamp
                .and_then(|t| chrono::DateTime::from_timestamp_millis(t))
                .map(|d| d.format("%H:%M:%S").to_string())
                .unwrap_or_default();
            let value_preview: String = msg.value.chars().take(60).collect();
            let style = if i == app.messages.selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };
            Row::new(vec![
                Cell::from(msg.partition.to_string()),
                Cell::from(msg.offset.to_string()),
                Cell::from(msg.key.as_deref().unwrap_or("").to_string()),
                Cell::from(value_preview),
                Cell::from(ts),
            ]).style(style)
        }).collect();

        let table = Table::new(rows, [
            Constraint::Length(6),
            Constraint::Length(10),
            Constraint::Length(15),
            Constraint::Min(30),
            Constraint::Length(12),
        ])
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Messages ({}) ", filtered.len()))
                .title_style(Style::default().fg(Color::Cyan))
                .border_style(Style::default().fg(if app.focus == Focus::Content { Color::Cyan } else { Color::DarkGray })),
        );
        frame.render_widget(table, chunks[2]);
    }

    // Help line
    let help = Paragraph::new(" s: Start/Stop | Enter: Detail | Esc: Back/Close | p: Produce | /: Filter | 1: Earliest | 2: Latest ")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[3]);
}
