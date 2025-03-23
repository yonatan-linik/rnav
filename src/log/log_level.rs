use ratatui::{style::Color, text::Span};

#[derive(Default, Debug, Clone, Copy)]
pub enum LogLevel {
    Warning,
    Error,
    #[default]
    Unknown,
}

impl std::str::FromStr for LogLevel {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "ERR" | "ERROR" => Ok(LogLevel::Error),
            "WARN" | "WARNING" => Ok(LogLevel::Warning),
            _ => Ok(LogLevel::Unknown),
        }
    }
}

impl From<LogLevel> for &str {
    fn from(val: LogLevel) -> Self {
        match val {
            LogLevel::Warning => " Warning ",
            LogLevel::Error => " Error ",
            LogLevel::Unknown => " Unknown ",
        }
    }
}

impl From<LogLevel> for Span<'static> {
    fn from(val: LogLevel) -> Self {
        let color = match val {
            LogLevel::Warning => Color::Yellow,
            LogLevel::Error => Color::Red,
            LogLevel::Unknown => Color::White,
        };

        let log_level_str: &'static str = val.into();

        Span::styled(log_level_str, color)
    }
}
