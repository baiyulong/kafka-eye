use anyhow::Result;
use ratatui::{backend::Backend, layout::Rect, Frame};
use crate::app::state::AppState;

pub fn render_topic_list<B: Backend>(f: &mut Frame<B>, area: Rect, state: &AppState) -> Result<()> {
    // TODO: Implement topic list view
    Ok(())
}

pub fn render_topic_detail<B: Backend>(f: &mut Frame<B>, area: Rect, state: &AppState) -> Result<()> {
    // TODO: Implement topic detail view
    Ok(())
}
