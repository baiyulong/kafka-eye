use crate::app::*;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_dialog(dialog: &Dialog, frame: &mut Frame) {
    match dialog {
        Dialog::CreateTopic(d) => render_create_topic(d, frame),
        Dialog::DeleteConfirm(d) => render_delete_confirm(d, frame),
        Dialog::ProduceMessage(d) => render_produce_message(d, frame),
        Dialog::ResetOffset(d) => render_reset_offset(d, frame),
        Dialog::EditCluster(d) => render_edit_cluster(d, frame),
        Dialog::ConnectionTest(d) => render_connection_test(d, frame),
    }
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

fn render_input_field(frame: &mut Frame, area: Rect, label: &str, value: &str, focused: bool) {
    let style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };
    let border_style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let display = if value.is_empty() && !focused {
        format!(" {}: (empty)", label)
    } else {
        format!(" {}: {}", label, value)
    };
    let widget = Paragraph::new(display)
        .style(style)
        .block(Block::default().borders(Borders::ALL).border_style(border_style));
    frame.render_widget(widget, area);
}

fn render_create_topic(dialog: &CreateTopicDialog, frame: &mut Frame) {
    let area = centered_rect(50, 50, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Create Topic ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .margin(1)
        .split(inner);

    render_input_field(frame, chunks[0], "Topic Name", &dialog.name, dialog.focused_field == 0);
    render_input_field(frame, chunks[1], "Partitions", &dialog.partitions, dialog.focused_field == 1);
    render_input_field(frame, chunks[2], "Replication Factor", &dialog.replication_factor, dialog.focused_field == 2);
    render_input_field(frame, chunks[3], "Retention (ms)", &dialog.retention_ms, dialog.focused_field == 3);

    let help = Paragraph::new(" Tab: Next Field | Enter: Create | Esc: Cancel ")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(help, chunks[4]);
}

fn render_delete_confirm(dialog: &DeleteConfirmDialog, frame: &mut Frame) {
    let area = centered_rect(50, 30, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ⚠ Confirm Delete ")
        .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(Color::Red));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .margin(1)
        .split(inner);

    let warning = Paragraph::new(format!(
        " Are you sure you want to delete {} '{}'?",
        dialog.item_type, dialog.item_name
    ))
    .style(Style::default().fg(Color::Yellow));
    frame.render_widget(warning, chunks[0]);

    let input = Paragraph::new(format!(" Type name to confirm: {}", dialog.confirm_input))
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow)));
    frame.render_widget(input, chunks[1]);

    let help = Paragraph::new(" Enter: Confirm | Esc: Cancel ")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(help, chunks[2]);
}

fn render_produce_message(dialog: &ProduceMessageDialog, frame: &mut Frame) {
    let area = centered_rect(60, 60, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Produce to: {} ", dialog.topic))
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .margin(1)
        .split(inner);

    render_input_field(frame, chunks[0], "Key (optional)", &dialog.key, dialog.focused_field == 0);

    // Value field (multi-line)
    let value_style = if dialog.focused_field == 1 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };
    let value_border = if dialog.focused_field == 1 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let value_widget = Paragraph::new(format!(" {}", if dialog.value.is_empty() { "(enter message value)" } else { &dialog.value }))
        .style(value_style)
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL).title(" Value ").border_style(value_border));
    frame.render_widget(value_widget, chunks[1]);

    render_input_field(frame, chunks[2], "Headers (k1:v1,k2:v2)", &dialog.headers, dialog.focused_field == 2);

    if let Some(ref result) = dialog.result_message {
        let result_color = if result.starts_with("✓") { Color::Green } else { Color::Red };
        let result_widget = Paragraph::new(format!(" {}", result))
            .style(Style::default().fg(result_color));
        frame.render_widget(result_widget, chunks[3]);
    } else {
        let help = Paragraph::new(" Tab: Next Field | Ctrl+Enter/F5: Send | Esc: Cancel ")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(help, chunks[3]);
    }
}

fn render_reset_offset(dialog: &ResetOffsetDialog, frame: &mut Frame) {
    let area = centered_rect(50, 30, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ⚠ Reset Offsets ")
        .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(Color::Red));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .margin(1)
        .split(inner);

    let info = Paragraph::new(format!(" Group: {} | Topic: {}", dialog.group_id, dialog.topic))
        .style(Style::default().fg(Color::White));
    frame.render_widget(info, chunks[0]);

    let target_text = match dialog.target {
        ResetTarget::Earliest => "→ Earliest",
        ResetTarget::Latest => "→ Latest",
    };
    let target = Paragraph::new(format!(" Target: {} (use ←/→ to change)", target_text))
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(target, chunks[1]);

    let help = Paragraph::new(" Enter: Confirm | Esc: Cancel | ←/→: Change target ")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(help, chunks[2]);
}

fn render_edit_cluster(dialog: &EditClusterDialog, frame: &mut Frame) {
    let area = centered_rect(60, 70, frame.area());
    frame.render_widget(Clear, area);

    let title = if dialog.editing_index.is_some() { " Edit Cluster " } else { " Add Cluster " };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let auth_types = ["None", "SASL/PLAIN", "SCRAM-SHA-256", "SCRAM-SHA-512", "SSL/TLS",
                      "SASL_SSL/PLAIN", "SASL_SSL/SCRAM-256", "SASL_SSL/SCRAM-512"];
    let auth_display = format!("{} (←/→ to change)", auth_types.get(dialog.auth_type).unwrap_or(&"Unknown"));

    let mut constraints = vec![
        Constraint::Length(3), // name
        Constraint::Length(3), // brokers
        Constraint::Length(3), // auth type
    ];

    let needs_credentials = matches!(dialog.auth_type, 1 | 2 | 3 | 5 | 6 | 7);
    let needs_ca = matches!(dialog.auth_type, 5 | 6 | 7);
    let needs_ssl_certs = dialog.auth_type == 4;

    if needs_credentials {
        constraints.push(Constraint::Length(3)); // username
        constraints.push(Constraint::Length(3)); // password
    }
    if needs_ca {
        constraints.push(Constraint::Length(3)); // ca_cert path (optional)
    }
    if needs_ssl_certs {
        constraints.push(Constraint::Length(3)); // ca_cert
        constraints.push(Constraint::Length(3)); // client_cert
        constraints.push(Constraint::Length(3)); // client_key
    }
    constraints.push(Constraint::Length(2)); // help
    constraints.push(Constraint::Min(0));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .margin(1)
        .split(inner);

    render_input_field(frame, chunks[0], "Cluster Name", &dialog.name, dialog.focused_field == 0);
    render_input_field(frame, chunks[1], "Brokers", &dialog.brokers, dialog.focused_field == 1);
    render_input_field(frame, chunks[2], "Auth Type", &auth_display, dialog.focused_field == 2);

    let mut field_idx = 3;
    if needs_credentials {
        render_input_field(frame, chunks[field_idx], "Username", &dialog.username, dialog.focused_field == 3);
        field_idx += 1;
        render_input_field(frame, chunks[field_idx], "Password", &"*".repeat(dialog.password.len()), dialog.focused_field == 4);
        field_idx += 1;
    }
    if needs_ca {
        render_input_field(frame, chunks[field_idx], "CA Cert Path (optional)", &dialog.ca_cert, dialog.focused_field == 5);
        field_idx += 1;
    }
    if needs_ssl_certs {
        render_input_field(frame, chunks[field_idx], "CA Cert Path", &dialog.ca_cert, dialog.focused_field == 3);
        field_idx += 1;
        render_input_field(frame, chunks[field_idx], "Client Cert Path", &dialog.client_cert, dialog.focused_field == 4);
        field_idx += 1;
        render_input_field(frame, chunks[field_idx], "Client Key Path", &dialog.client_key, dialog.focused_field == 5);
        field_idx += 1;
    }

    let help = Paragraph::new(" Tab: Next | Enter: Save | Esc: Cancel ")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(help, chunks[field_idx]);
}

fn render_connection_test(dialog: &ConnectionTestDialog, frame: &mut Frame) {
    let area = centered_rect(50, 20, frame.area());
    frame.render_widget(Clear, area);

    let (title_style, border_style) = match &dialog.status {
        ConnectionTestStatus::Testing => (Style::default().fg(Color::Yellow), Style::default().fg(Color::Yellow)),
        ConnectionTestStatus::Success(_) => (Style::default().fg(Color::Green), Style::default().fg(Color::Green)),
        ConnectionTestStatus::Failed(_) => (Style::default().fg(Color::Red), Style::default().fg(Color::Red)),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Connection Test: {} ", dialog.cluster_name))
        .title_style(title_style.add_modifier(Modifier::BOLD))
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let text = match &dialog.status {
        ConnectionTestStatus::Testing => " ⏳ Testing connection...".to_string(),
        ConnectionTestStatus::Success(msg) => format!(" ✓ {}", msg),
        ConnectionTestStatus::Failed(msg) => format!(" ✗ {}", msg),
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2), Constraint::Min(0)])
        .margin(1)
        .split(inner);

    let result = Paragraph::new(text).style(title_style);
    frame.render_widget(result, chunks[0]);

    let help = Paragraph::new(" Esc: Close ")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(help, chunks[1]);
}
