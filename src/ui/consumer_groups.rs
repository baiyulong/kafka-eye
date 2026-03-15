use crate::app::*;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_consumer_groups(app: &App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(area);

    let title = Paragraph::new(" Consumer Groups")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(title, chunks[0]);

    let header = Row::new(vec![
        Cell::from("Group Name").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("State").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Members").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Total Lag").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]).height(1);

    let rows: Vec<Row> = app.consumer_groups.groups.iter().enumerate().map(|(i, group)| {
        let state_color = match group.state.to_lowercase().as_str() {
            "stable" => Color::Green,
            "empty" => Color::Yellow,
            "dead" => Color::Red,
            "preparingrebalance" | "completingrebalance" => Color::Yellow,
            _ => Color::White,
        };
        let style = if i == app.consumer_groups.selected {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };
        Row::new(vec![
            Cell::from(group.name.clone()),
            Cell::from(Span::styled(&group.state, Style::default().fg(state_color))),
            Cell::from(group.members.to_string()),
            Cell::from(group.total_lag.to_string()),
        ]).style(style)
    }).collect();

    let table = Table::new(rows, [
        Constraint::Min(30),
        Constraint::Length(20),
        Constraint::Length(10),
        Constraint::Length(12),
    ])
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Consumer Groups ({}) ", app.consumer_groups.groups.len()))
            .title_style(Style::default().fg(Color::Cyan))
            .border_style(Style::default().fg(if app.focus == Focus::Content { Color::Cyan } else { Color::DarkGray })),
    );
    frame.render_widget(table, chunks[1]);

    let help = Paragraph::new(" Enter: Detail | r: Refresh | Esc: Back ")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
}

pub fn render_consumer_group_detail(app: &App, frame: &mut Frame, area: Rect, group_name: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(area);

    let title = Paragraph::new(format!(" Consumer Group: {}", group_name))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(title, chunks[0]);

    // Group info
    let group = app.consumer_groups.groups.iter().find(|g| g.name == group_name);
    if let Some(group) = group {
        let info_text = vec![
            Line::from(vec![
                Span::styled("State: ", Style::default().fg(Color::DarkGray)),
                Span::styled(&group.state, Style::default().fg(Color::Green)),
                Span::styled("  Members: ", Style::default().fg(Color::DarkGray)),
                Span::styled(group.members.to_string(), Style::default().fg(Color::White)),
                Span::styled("  Total Lag: ", Style::default().fg(Color::DarkGray)),
                Span::styled(group.total_lag.to_string(), Style::default().fg(
                    if group.total_lag > 1000 { Color::Red } else if group.total_lag > 0 { Color::Yellow } else { Color::Green }
                )),
            ]),
        ];
        let info = Paragraph::new(info_text)
            .block(Block::default().borders(Borders::ALL).title(" Info ").border_style(Style::default().fg(Color::DarkGray)));
        frame.render_widget(info, chunks[1]);

        // Partition lag table
        let header = Row::new(vec![
            Cell::from("Topic").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Partition").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Current Offset").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Log End Offset").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Lag").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]).height(1);

        let rows: Vec<Row> = group.lag.iter().map(|lag| {
            let lag_color = if lag.lag > 1000 { Color::Red } else if lag.lag > 0 { Color::Yellow } else { Color::Green };
            Row::new(vec![
                Cell::from(lag.topic.clone()),
                Cell::from(lag.partition.to_string()),
                Cell::from(lag.current_offset.to_string()),
                Cell::from(lag.log_end_offset.to_string()),
                Cell::from(Span::styled(lag.lag.to_string(), Style::default().fg(lag_color))),
            ])
        }).collect();

        let table = Table::new(rows, [
            Constraint::Min(20),
            Constraint::Length(10),
            Constraint::Length(16),
            Constraint::Length(16),
            Constraint::Length(10),
        ])
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Partition Lag ")
                .title_style(Style::default().fg(Color::Cyan))
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(table, chunks[2]);
    } else {
        let empty = Paragraph::new("Loading group details...")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(empty, chunks[1]);
    }

    let help = Paragraph::new(" Esc: Back | r: Refresh ")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[3]);
}
