use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEventKind};
use tokio::sync::mpsc;

#[derive(Debug)]
#[allow(dead_code)]
pub enum Event {
    Key(crossterm::event::KeyEvent),
    Tick,
    Resize(u16, u16),
}

pub struct EventHandler {
    receiver: mpsc::UnboundedReceiver<Event>,
    _task: tokio::task::JoinHandle<()>,
}

impl EventHandler {
    pub fn new(tick_rate_ms: u64) -> Self {
        let tick_rate = Duration::from_millis(tick_rate_ms);
        let (sender, receiver) = mpsc::unbounded_channel();

        let _task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tick_rate);
            loop {
                let event = tokio::select! {
                    _ = interval.tick() => Event::Tick,
                    _ = tokio::task::spawn_blocking(move || {
                        event::poll(Duration::from_millis(0)).ok()
                    }) => {
                        if event::poll(Duration::ZERO).unwrap_or(false) {
                            match event::read() {
                                Ok(CrosstermEvent::Key(key)) if key.kind == KeyEventKind::Press => {
                                    Event::Key(key)
                                }
                                Ok(CrosstermEvent::Resize(w, h)) => Event::Resize(w, h),
                                _ => continue,
                            }
                        } else {
                            continue;
                        }
                    }
                };

                if sender.send(event).is_err() {
                    break;
                }
            }
        });

        Self { receiver, _task }
    }

    pub async fn next(&mut self) -> anyhow::Result<Event> {
        self.receiver
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("Event channel closed"))
    }
}
