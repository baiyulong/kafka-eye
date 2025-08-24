use anyhow::Result;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::state::{AppState, ClusterFormAction};

pub fn render_cluster_management(f: &mut Frame, area: Rect, state: &AppState) -> Result<()> {
    if state.mode == crate::app::state::AppMode::ClusterForm {
        render_cluster_form(f, area, state)?;
    } else {
        render_cluster_list(f, area, state)?;
    }
    Ok(())
}

fn render_cluster_list(f: &mut Frame, area: Rect, state: &AppState) -> Result<()> {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(5),    // List
            Constraint::Length(5), // Help
        ])
        .split(area);

    // Title
    let title = Paragraph::new("Cluster Management")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(title, chunks[0]);

    // Cluster list
    let mut items = Vec::new();
    
    if state.cluster_list.is_empty() {
        items.push(ListItem::new("No clusters configured"));
    } else {
        for (i, cluster) in state.cluster_list.iter().enumerate() {
            let style = if i == state.selected_index {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            };
            
            let item = ListItem::new(cluster.as_str()).style(style);
            items.push(item);
        }
        
        // Add "Add new cluster" option
        let add_style = if state.selected_index == state.cluster_list.len() {
            Style::default().bg(Color::Green).fg(Color::White)
        } else {
            Style::default().fg(Color::Green)
        };
        items.push(ListItem::new("+ Add new cluster").style(add_style));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Clusters"));
    f.render_widget(list, chunks[1]);

    // Help text
    let help_text = vec![
        Line::from(vec![
            Span::styled("Navigation: ", Style::default().fg(Color::Yellow)),
            Span::raw("↑/↓ or j/k - Move, "),
            Span::styled("a", Style::default().fg(Color::Green)),
            Span::raw(" - Add, "),
            Span::styled("e/Enter", Style::default().fg(Color::Blue)),
            Span::raw(" - Edit, "),
        ]),
        Line::from(vec![
            Span::styled("d/Delete", Style::default().fg(Color::Red)),
            Span::raw(" - Delete, "),
            Span::styled("s", Style::default().fg(Color::Cyan)),
            Span::raw(" - Switch to cluster, "),
            Span::styled("Esc", Style::default().fg(Color::Gray)),
            Span::raw(" - Back"),
        ]),
    ];

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .wrap(Wrap { trim: true });
    f.render_widget(help, chunks[2]);

    Ok(())
}

fn render_cluster_form(f: &mut Frame, area: Rect, state: &AppState) -> Result<()> {
    // Create a popup in the center
    let popup_area = centered_rect(80, 80, area);
    
    // Clear the background
    f.render_widget(Clear, popup_area);
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(10),   // Form
            Constraint::Length(3), // Instructions
        ])
        .split(popup_area);

    // Title
    let title_text = match state.cluster_form_action {
        ClusterFormAction::Add => "Add New Cluster",
        ClusterFormAction::Edit => "Edit Cluster",
        ClusterFormAction::Delete => "Delete Cluster",
    };
    
    let title = Paragraph::new(title_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(title, chunks[0]);

    // Form content
    match state.cluster_form_action {
        ClusterFormAction::Delete => {
            render_delete_confirmation(f, chunks[1], state)?;
        }
        _ => {
            render_form_fields(f, chunks[1], state)?;
        }
    }

    // Instructions
    let instructions = match state.cluster_form_action {
        ClusterFormAction::Delete => {
            Paragraph::new("Press Enter to confirm deletion, Esc to cancel")
        }
        _ => {
            Paragraph::new("Tab/Shift+Tab: Navigate fields, Enter: Submit, Esc: Cancel")
        }
    };
    
    let instructions = instructions
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));
    f.render_widget(instructions, chunks[2]);

    Ok(())
}

fn render_delete_confirmation(f: &mut Frame, area: Rect, state: &AppState) -> Result<()> {
    let text = format!(
        "Are you sure you want to delete cluster '{}'?\n\nThis action cannot be undone.",
        state.cluster_form.name
    );
    
    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Confirm Deletion"))
        .style(Style::default().fg(Color::Red))
        .wrap(Wrap { trim: true });
    
    f.render_widget(paragraph, area);
    Ok(())
}

fn render_form_fields(f: &mut Frame, area: Rect, state: &AppState) -> Result<()> {
    let field_constraints = vec![
        Constraint::Length(3); 8 // 8 fields
    ];
    
    let field_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(field_constraints)
        .split(area);

    let fields = [
        ("Cluster Name *", &state.cluster_form.name),
        ("Brokers * (comma-separated)", &state.cluster_form.brokers),
        ("Client ID", &state.cluster_form.client_id),
        ("Security Protocol", &state.cluster_form.security_protocol),
        ("SASL Mechanism", &state.cluster_form.sasl_mechanism),
        ("SASL Username", &state.cluster_form.sasl_username),
        ("SASL Password", &state.cluster_form.sasl_password),
        ("SSL CA Location", &state.cluster_form.ssl_ca_location),
    ];

    for (i, (label, value)) in fields.iter().enumerate() {
        if i < field_chunks.len() {
            let is_current = i == state.cluster_form.current_field;
            let style = if is_current {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            };
            
            let border_style = if is_current {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            let display_value = if label.contains("Password") && !value.is_empty() {
                "*".repeat(value.len())
            } else {
                value.to_string()
            };

            let field = Paragraph::new(display_value)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title(*label)
                    .border_style(border_style))
                .style(style);
            
            f.render_widget(field, field_chunks[i]);
        }
    }

    Ok(())
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
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
