use anyhow::Result;
use ratatui::{backend::Backend, layout::Rect, Frame};
use crate::app::state::AppState;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) -> Result<()> {
    // TODO: Implement settings view
    Ok(())
}
