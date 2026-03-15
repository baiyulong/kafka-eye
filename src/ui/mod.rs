mod sidebar;
mod dashboard;
mod topics;
mod messages;
mod consumer_groups;
mod dialogs;
mod help;

use crate::app::*;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render(app: &App, frame: &mut Frame) {
    if app.route == Route::ClusterSelect {
        render_cluster_select(app, frame);
        if let Some(ref dialog) = app.dialog {
            dialogs::render_dialog(dialog, frame);
        }
        if app.show_help {
            help::render_help(frame);
        }
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(24),
            Constraint::Min(40),
        ])
        .split(chunks[0]);

    sidebar::render_sidebar(app, frame, main_chunks[0]);
    render_content(app, frame, main_chunks[1]);
    render_status_bar(app, frame, chunks[1]);

    if let Some(ref dialog) = app.dialog {
        dialogs::render_dialog(dialog, frame);
    }

    if app.show_help {
        help::render_help(frame);
    }
}

fn render_cluster_select(app: &App, frame: &mut Frame) {
    let area = frame.area();
    frame.render_widget(Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .margin(2)
        .split(area);

    // Title
    let title = Paragraph::new("🔑 Kafka Eye - Select Cluster")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(title, chunks[0]);

    // Cluster list
    if app.config.clusters.is_empty() {
        let empty = Paragraph::new("No clusters configured.\nPress 'a' to add a new cluster, '?' for help.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(" Clusters ").border_style(Style::default().fg(Color::DarkGray)));
        frame.render_widget(empty, chunks[1]);
    } else {
        let items: Vec<ListItem> = app.config.clusters.iter().enumerate().map(|(i, c)| {
            let auth_badge = match &c.auth {
                crate::config::AuthConfig::None => "🔓",
                crate::config::AuthConfig::SaslPlain { .. } => "🔐 SASL",
                crate::config::AuthConfig::SaslScram256 { .. } => "🔐 SCRAM-256",
                crate::config::AuthConfig::SaslScram512 { .. } => "🔐 SCRAM-512",
                crate::config::AuthConfig::Ssl { .. } => "🔒 SSL",
            };
            let style = if i == app.cluster_select_index {
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("  {} ", c.name), style),
                Span::styled(format!("({}) ", c.brokers), Style::default().fg(Color::DarkGray)),
                Span::styled(auth_badge, Style::default().fg(Color::Yellow)),
            ]))
        }).collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" Clusters ").border_style(Style::default().fg(Color::Cyan)))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));
        frame.render_widget(list, chunks[1]);
    }

    // Help bar
    let help_text = Paragraph::new(" Enter: Connect | a: Add | e: Edit | d: Delete | t: Test | ?: Help | q: Quit ")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(help_text, chunks[2]);
}

fn render_content(app: &App, frame: &mut Frame, area: Rect) {
    match &app.route {
        Route::Dashboard => dashboard::render_dashboard(app, frame, area),
        Route::Topics => topics::render_topics(app, frame, area),
        Route::TopicDetail(name) => topics::render_topic_detail(app, frame, area, name),
        Route::Messages(topic) => messages::render_messages(app, frame, area, topic),
        Route::ConsumerGroups => consumer_groups::render_consumer_groups(app, frame, area),
        Route::ConsumerGroupDetail(name) => consumer_groups::render_consumer_group_detail(app, frame, area, name),
        _ => {}
    }
}

fn render_status_bar(app: &App, frame: &mut Frame, area: Rect) {
    let cluster_name = app.active_cluster_config()
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "Not connected".to_string());

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(30),
            Constraint::Min(20),
            Constraint::Length(20),
        ])
        .split(area);

    let cluster_info = Paragraph::new(format!(" 📡 {}", cluster_name))
        .style(Style::default().fg(Color::Green))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(cluster_info, chunks[0]);

    let status = Paragraph::new(format!(" {}", app.status_message))
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(status, chunks[1]);

    let time = chrono::Local::now().format("%H:%M:%S").to_string();
    let time_widget = Paragraph::new(format!(" {} ", time))
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Right)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(time_widget, chunks[2]);
}
