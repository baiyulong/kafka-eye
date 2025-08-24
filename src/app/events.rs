use crossterm::event::{KeyEvent, MouseEvent};

#[derive(Debug, Clone)]
pub enum AppEvent {
    Input(InputEvent),
    Tick,
    KafkaEvent(crate::kafka::KafkaEvent),
}

#[derive(Debug, Clone)]
pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
}
