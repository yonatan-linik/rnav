use chrono::{DateTime, FixedOffset};
use ratatui::prelude::{self, Line};

#[derive(Debug, Clone, Copy)]
pub struct LogLine<'a> {
    pub time: DateTime<FixedOffset>,
    pub log: &'a str,
}

impl<'a> LogLine<'a> {
    pub fn new<S: AsRef<str> + ?Sized>(line: &'a S) -> Option<Self> {
        let log = line.as_ref();
        let end_of_rfc2822_time_index = log.find('+').map(|i| i + 5).unwrap_or(0);

        let (Ok(time) | Err(Ok(time))) =
            DateTime::parse_from_rfc2822(&log[..end_of_rfc2822_time_index]).map_err(|_| {
                DateTime::parse_from_rfc3339(
                    log.split_whitespace()
                        .next()
                        .expect("Log line to not be empty"),
                )
            })
        else {
            return None;
        };

        Some(LogLine { time, log })
    }
}

impl<'a> Into<Line<'a>> for LogLine<'a> {
    fn into(self) -> Line<'a> {
        Line::raw(self.log)
    }
}
