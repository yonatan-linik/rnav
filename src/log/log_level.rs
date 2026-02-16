use ratatui::{
    style::{Color, Style},
    text::Span,
};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
    Notice,
    Critical,
    #[default]
    Unknown,
}

impl std::str::FromStr for LogLevel {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "TRACE" => Ok(Self::Trace),
            "DEBUG" => Ok(Self::Debug),
            "INFO" => Ok(Self::Info),
            "WARN" | "WARNING" => Ok(Self::Warning),
            "ERR" | "ERROR" => Ok(Self::Error),
            "NOTICE" => Ok(Self::Notice),
            "CRIT" | "CRITICAL" => Ok(Self::Critical),
            _ => Ok(Self::Unknown),
        }
    }
}

impl From<LogLevel> for &str {
    fn from(val: LogLevel) -> Self {
        match val {
            LogLevel::Trace => " Trace ",
            LogLevel::Debug => " Debug ",
            LogLevel::Info => " Info ",
            LogLevel::Warning => " Warning ",
            LogLevel::Error => " Error ",
            LogLevel::Notice => " Notice ",
            LogLevel::Critical => " Critical ",
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
            LogLevel::Trace => Color::DarkGray,
            LogLevel::Debug => Color::Cyan,
            LogLevel::Info => Color::Green,
            LogLevel::Notice => Color::LightRed,
            LogLevel::Critical => Color::Red,
        };

        let style = match val {
            LogLevel::Notice | LogLevel::Critical => Style::new().fg(color).slow_blink(),
            _ => Style::new().fg(color),
        };

        let log_level_str: &'static str = val.into();

        Span::styled(log_level_str, style)
    }
}
