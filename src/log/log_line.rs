use std::borrow::Cow;

use chrono::{DateTime, FixedOffset};

use crate::log::log_file::LogFile;
use crate::log::log_level::LogLevel;

thread_local! {
    static LOG_LEVEL_REGEX: std::cell::LazyCell<regex::Regex> =
        std::cell::LazyCell::new(|| regex::Regex::new(r"([^\w]|^)(?<level>TRACE|DEBUG|INFO|ERR|ERROR|WARN|WARNING|NOTICE|CRIT|CRITICAL)([^\w]|$)")
            .expect("Legal regex"));
}

#[derive(Debug, Clone)]
pub struct LogLine<'a> {
    pub src_file: &'a LogFile,
    pub time: DateTime<FixedOffset>,
    pub log: Cow<'a, str>,
    pub level: LogLevel,
    pub marked: bool,
    pub comment: Option<String>,
}

impl PartialEq for LogLine<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.src_file.name == other.src_file.name
            && self.time == other.time
            && self.log == other.log
            && self.marked == other.marked
            && self.comment == other.comment
    }
}

impl Eq for LogLine<'_> {}

impl<'a> LogLine<'a> {
    /// Tries to construct a `LogLine` from a line of text.
    /// The line must contain a timestamp in either RFC2822 or RFC3339 format.
    /// If there is no timestamp, `None` is returned.
    ///
    /// # Examples
    /// ```rust
    /// # use std::path::Path;
    /// # use rnav::LogLine;
    /// # use rnav::log::log_file::LogFile;
    /// # use rnav::log::log_level::LogLevel;
    /// # use chrono::offset::FixedOffset;
    /// # use chrono::DateTime;
    /// # use chrono::TimeZone;
    /// # use chrono::Utc;
    /// std::fs::write("/tmp/test",
    ///                "2021-08-01T12:00:00Z INFO Hello, world!
    ///                 Wed, 18 Feb 2015 23:16:09 GMT ERROR Log contents
    ///                 18/Mar/2003:08:05:30 +0200 Unknown format").unwrap();
    /// let file = LogFile::new(Path::new("/tmp/test").into());
    /// let contents = file.contents();
    /// let mut lines = contents.split(|c| *c == b'\n').map(|l| str::from_utf8(l).unwrap().trim().into());
    /// // RFC3339 format
    /// assert_eq!(LogLine::new(&file, lines.next().unwrap()),
    ///            LogLine { src_file: &file,
    ///                      time: FixedOffset::east_opt(0)
    ///                            .unwrap()
    ///                            .with_ymd_and_hms(2021, 8, 1, 12, 0, 0)
    ///                            .unwrap(),
    ///                      log: "2021-08-01T12:00:00Z INFO Hello, world!".into(),
    ///                      level: LogLevel::Unknown,
    ///                      marked: false,
    ///                      comment: None
    ///                    }
    ///           );
    /// // RFC2822 format
    /// assert_eq!(LogLine::new(&file, lines.next().unwrap()),
    ///            LogLine { src_file: &file,
    ///                      time: FixedOffset::east_opt(0)
    ///                            .unwrap()
    ///                            .with_ymd_and_hms(2015, 2, 18, 23, 16, 9)
    ///                            .unwrap(),
    ///                      log: "Wed, 18 Feb 2015 23:16:09 GMT ERROR Log contents".into(),
    ///                      level: LogLevel::Error,
    ///                      marked: false,
    ///                      comment: None
    ///                    }
    ///           );
    /// // Unknown time format
    /// assert_eq!(LogLine::new(&file, lines.next().unwrap()),
    ///            LogLine { src_file: &file,
    ///                      time: DateTime::<Utc>::MAX_UTC.fixed_offset(),
    ///                      log: "18/Mar/2003:08:05:30 +0200 Unknown format".into(),
    ///                      level: LogLevel::Unknown,
    ///                      marked: false,
    ///                      comment: None
    ///                    }
    ///           );
    ///
    /// # std::fs::remove_file("/tmp/test").unwrap();
    /// ```
    pub fn new(src_file: &'a LogFile, line: Cow<'a, str>) -> Self {
        let log = line.trim();
        let end_of_rfc2822_time_index = log
            .find('+')
            .map(|i| i + 5)
            .or_else(|| log.find("GMT").map(|i| i + 3))
            .unwrap_or(0);

        let find_time =
            DateTime::parse_from_rfc2822(&log[..end_of_rfc2822_time_index]).map_err(|_| {
                DateTime::parse_from_rfc3339(
                    log.split_whitespace()
                        .next()
                        .expect("Log line to not be empty"),
                )
            });

        let time = match find_time {
            Ok(time) | Err(Ok(time)) => time,
            _ => chrono::DateTime::<chrono::Utc>::MAX_UTC.fixed_offset(),
        };

        // log level regex
        let level = LOG_LEVEL_REGEX.with(|r| {
            r.captures_iter(log)
                .next()
                .and_then(|m| m.name("level").map(|l| l.as_str().parse().ok()))
                .flatten()
                .unwrap_or_default()
        });

        LogLine {
            src_file,
            time,
            log: line,
            level,
            marked: false,
            comment: None,
        }
    }
}

impl<'a> From<LogLine<'a>> for ratatui::prelude::Line<'a> {
    fn from(val: LogLine<'a>) -> Self {
        Self::raw(val.log)
    }
}
