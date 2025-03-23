use chrono::{DateTime, FixedOffset};

use crate::log_file::LogFile;

#[derive(Debug, Clone)]
pub struct LogLine<'a> {
    pub src_file: &'a LogFile,
    pub time: DateTime<FixedOffset>,
    pub log: &'a str,
    pub marked: bool,
}

impl PartialEq for LogLine<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.src_file.name == other.src_file.name
            && self.time == other.time
            && self.log == other.log
            && self.marked == other.marked
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
    /// # use rnav::LogLine;
    /// # use rnav::log_file::LogFile;
    /// # use chrono::offset::FixedOffset;
    /// # use chrono::DateTime;
    /// # use chrono::TimeZone;
    /// std::fs::write("/tmp/test",
    ///                "2021-08-01T12:00:00Z INFO Hello, world!
    ///                 Wed, 18 Feb 2015 23:16:09 GMT Log contents
    ///                 18/Mar/2003:08:05:30 +0200 Unknown format").unwrap();
    /// let file = LogFile::new_with_random_color("/tmp/test".into());
    /// let contents = file.contents();
    /// let mut lines = contents.lines();
    /// // RFC3339 format
    /// assert_eq!(LogLine::new(&file, lines.next().unwrap()),
    ///            Some(LogLine { src_file: &file,
    ///                           time: FixedOffset::east_opt(0)
    ///                             .unwrap()
    ///                             .with_ymd_and_hms(2021, 8, 1, 12, 0, 0)
    ///                             .unwrap(),
    ///                           log: "2021-08-01T12:00:00Z INFO Hello, world!", marked: false
    ///                         })
    ///           );
    /// // RFC2822 format
    /// assert_eq!(LogLine::new(&file, lines.next().unwrap()),
    ///            Some(LogLine { src_file: &file,
    ///                           time: FixedOffset::east_opt(0)
    ///                             .unwrap()
    ///                             .with_ymd_and_hms(2015, 2, 18, 23, 16, 9)
    ///                             .unwrap(),
    ///                           log: "Wed, 18 Feb 2015 23:16:09 GMT Log contents", marked: false
    ///                         })
    ///           );
    /// // Unknown format
    /// assert_eq!(LogLine::new(&file, lines.next().unwrap()), None);
    ///
    /// # std::fs::remove_file("/tmp/test").unwrap();
    /// ```
    pub fn new<S: AsRef<str> + ?Sized>(src_file: &'a LogFile, line: &'a S) -> Option<Self> {
        let log = line.as_ref().trim();
        let end_of_rfc2822_time_index = log
            .find('+')
            .map(|i| i + 5)
            .or_else(|| log.find("GMT").map(|i| i + 3))
            .unwrap_or(0);

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

        Some(LogLine {
            src_file,
            time,
            log,
            marked: false,
        })
    }
}

impl<'a> From<LogLine<'a>> for ratatui::prelude::Line<'a> {
    fn from(val: LogLine<'a>) -> Self {
        Self::raw(val.log)
    }
}
