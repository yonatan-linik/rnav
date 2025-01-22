use rand::Rng as _;
use ratatui::style::Color;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogFile {
    pub name: Arc<str>,
    pub contents: String,
    pub color: Color,
}

impl LogFile {
    pub fn new_with_random_color(name: Arc<str>, contents: String) -> Self {
        let color = Color::from_u32(rand::thread_rng().gen_range(255..=0x00FF_FFFF));
        Self {
            name,
            contents,
            color,
        }
    }
}
