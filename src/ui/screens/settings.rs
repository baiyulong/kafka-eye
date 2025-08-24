use anyhow::Result;
use ratatui::{backend::Backend, layout::Rect, Frame};
use crate::app::state::AppState;

pub fn render<B: Backend>(f: &mut Frame<B>, area: Rect, state: &AppState) -> Result<()> {
    // TODO: Implement settings view
    Ok(())
}
