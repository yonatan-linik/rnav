use crate::error::{Error, Result};
use strum::EnumIter;
use strum::IntoEnumIterator;

#[derive(Debug, EnumIter, Copy, Clone)]
pub enum Command {
    FilterIn,
    FilterOut,
    Highlight,
}

impl std::str::FromStr for Command {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "filter-in" => Ok(Command::FilterIn),
            "filter-out" => Ok(Command::FilterOut),
            "highlight" => Ok(Command::Highlight),
            _ => Err(Error::UnknownCommand(s.to_string())),
        }
    }
}

impl From<Command> for &'static str {
    fn from(command: Command) -> Self {
        match command {
            Command::FilterIn => "filter-in",
            Command::FilterOut => "filter-out",
            Command::Highlight => "highlight",
        }
    }
}

impl Command {
    fn as_str(&self) -> &'static str {
        (*self).into()
    }

    pub fn auto_complete(prefix: &str) -> (Option<String>, Vec<&'static str>) {
        let completions: Vec<&'static str> = Command::iter()
            .filter_map(|c| c.as_str().starts_with(prefix).then_some(c.as_str()))
            .collect();

        let longest_common_prefix =
            completions
                .iter()
                .fold(None, |acc: Option<String>, &s| match acc {
                    Some(mut prefix) => {
                        while !s.starts_with(&prefix) {
                            let len = prefix.len();
                            if len == 0 {
                                return None;
                            }
                            prefix.truncate(len - 1);
                        }
                        Some(prefix)
                    }
                    None => Some(s.to_string()),
                });

        (longest_common_prefix, completions)
    }
}
