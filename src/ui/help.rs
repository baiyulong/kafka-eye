use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_help(frame: &mut Frame) {
    let area = frame.area();
    let popup_area = centered_rect(60, 70, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ❓ Keyboard Shortcuts ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let help_text = vec![
        Line::from(Span::styled("Navigation", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Tab       ", Style::default().fg(Color::Cyan)),
            Span::raw("Switch focus between sidebar and content"),
        ]),
        Line::from(vec![
            Span::styled("  j/↓       ", Style::default().fg(Color::Cyan)),
            Span::raw("Move down in list"),
        ]),
        Line::from(vec![
            Span::styled("  k/↑       ", Style::default().fg(Color::Cyan)),
            Span::raw("Move up in list"),
        ]),
        Line::from(vec![
            Span::styled("  Enter     ", Style::default().fg(Color::Cyan)),
            Span::raw("Select / Drill into"),
        ]),
        Line::from(vec![
            Span::styled("  Esc       ", Style::default().fg(Color::Cyan)),
            Span::raw("Go back / Close dialog"),
        ]),
        Line::from(vec![
            Span::styled("  /         ", Style::default().fg(Color::Cyan)),
            Span::raw("Focus search bar"),
        ]),
        Line::from(vec![
            Span::styled("  ?         ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle this help"),
        ]),
        Line::from(""),
        Line::from(Span::styled("Topics", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  c         ", Style::default().fg(Color::Cyan)),
            Span::raw("Create new topic"),
        ]),
        Line::from(vec![
            Span::styled("  d         ", Style::default().fg(Color::Cyan)),
            Span::raw("Delete selected topic"),
        ]),
        Line::from(vec![
            Span::styled("  m         ", Style::default().fg(Color::Cyan)),
            Span::raw("Browse messages for topic"),
        ]),
        Line::from(vec![
            Span::styled("  r         ", Style::default().fg(Color::Cyan)),
            Span::raw("Refresh data"),
        ]),
        Line::from(""),
        Line::from(Span::styled("Messages", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  s         ", Style::default().fg(Color::Cyan)),
            Span::raw("Start/stop consuming"),
        ]),
        Line::from(vec![
            Span::styled("  p         ", Style::default().fg(Color::Cyan)),
            Span::raw("Produce a message"),
        ]),
        Line::from(vec![
            Span::styled("  1         ", Style::default().fg(Color::Cyan)),
            Span::raw("Consume from earliest"),
        ]),
        Line::from(vec![
            Span::styled("  2         ", Style::default().fg(Color::Cyan)),
            Span::raw("Consume from latest"),
        ]),
        Line::from(""),
        Line::from(Span::styled("General", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  q         ", Style::default().fg(Color::Cyan)),
            Span::raw("Quit application"),
        ]),
    ];

    let help = Paragraph::new(help_text)
        .wrap(Wrap { trim: false });
    frame.render_widget(help, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
