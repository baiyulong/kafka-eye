use anyhow::Result;
use ratatui::{backend::Backend, layout::Rect, Frame};
use crate::app::state::AppState;

pub fn render_producer(f: &mut Frame, area: Rect, state: &AppState) -> Result<()> {
    // TODO: Implement message producer view
    Ok(())
}

pub fn render_consumer(f: &mut Frame, area: Rect, state: &AppState) -> Result<()> {
    // TODO: Implement message consumer view
    Ok(())
}
