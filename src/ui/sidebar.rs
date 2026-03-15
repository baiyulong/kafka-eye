use crate::app::*;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_sidebar(app: &App, frame: &mut Frame, area: Rect) {
    let is_focused = app.focus == Focus::Sidebar;
    let border_color = if is_focused { Color::Cyan } else { Color::DarkGray };

    let items: Vec<ListItem> = app.sidebar.items.iter().enumerate().map(|(i, item)| {
        let prefix = if i == app.sidebar.selected && is_focused { "▸ " } else { "  " };
        let indent = "  ".repeat(item.indent);
        let icon = match &item.route {
            Route::Dashboard => "📊",
            Route::Topics => "📋",
            Route::ConsumerGroups => "👥",
            _ => "  ",
        };
        let style = if i == app.sidebar.selected {
            if is_focused {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            }
        } else {
            Style::default().fg(Color::Gray)
        };
        ListItem::new(format!("{}{}{} {}", prefix, indent, icon, item.label)).style(style)
    }).collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Navigation ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .border_style(Style::default().fg(border_color))
        );

    frame.render_widget(list, area);
}
