use std::collections::HashMap;

use crate::app_state::AppMode;
use crate::error::{Error, Result};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Color, Modifier, Stylize as _};
use ratatui::text::{Line, Span, Text};
use strum::EnumCount;

#[derive(Debug, EnumCount, Clone)]
pub enum Command {
    FilterIn(regex::Regex),
    FilterOut(regex::Regex),
    Highlight(regex::Regex),
}

thread_local! {
    static COMMAND_NAMES: std::cell::LazyCell<[&'static str; Command::COUNT]> =
        std::cell::LazyCell::new(|| ["filter-in", "filter-out", "highlight"]);
}

type CommandBuilder = dyn Fn(regex::Regex) -> Command;

impl std::str::FromStr for Command {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let map: [(&str, Box<CommandBuilder>); Command::COUNT] = [
            ("filter-in", Box::new(Command::FilterIn)),
            ("filter-out", Box::new(Command::FilterOut)),
            ("highlight", Box::new(Command::Highlight)),
        ];

        let map: HashMap<&str, Box<dyn Fn(regex::Regex) -> Command>> = HashMap::from(map);

        let (command, args) = s
            .trim()
            .split_once(|c: char| c.is_whitespace())
            .unwrap_or((s.trim(), ""));

        let build_command = map
            .get(command)
            .ok_or_else(|| Error::UnknownCommand(command.to_string()))?;

        // Currently all commands need arguments
        if args.is_empty() {
            return Err(Error::NoArgumentsGivenToCommand);
        }

        let r = regex::Regex::new(args)?;

        Ok(build_command(r))
    }
}

impl From<&Command> for &'static str {
    fn from(command: &Command) -> Self {
        match command {
            Command::FilterIn(_) => "filter-in",
            Command::FilterOut(_) => "filter-out",
            Command::Highlight(_) => "highlight",
        }
    }
}

impl Command {
    pub fn auto_complete(prefix: &str) -> (Option<String>, Vec<&'static str>) {
        let completions: Vec<&'static str> = COMMAND_NAMES.with(|names| {
            names
                .iter()
                .filter_map(|&c| c.starts_with(prefix).then_some(c))
                .collect()
        });

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

pub struct Commands {
    current_command: String,
    command_completions: Vec<&'static str>,
    command_error: String,
}

impl Commands {
    pub fn new() -> Self {
        Self {
            current_command: String::new(),
            command_completions: vec![],
            command_error: String::new(),
        }
    }

    pub fn exit_command_mode(&mut self) {
        self.command_completions.clear();
        self.command_error.clear();
        self.current_command.clear();
    }

    fn command_bar_completions(&self) -> Option<Line<'_>> {
        if self.command_completions.is_empty() {
            return None;
        }

        Some(Line::from_iter(self.command_completions.iter().flat_map(
            |c| [Span::styled(*c, Color::White), Span::raw(" ")],
        )))
    }

    pub fn command_bar_text(&self, app_mode: AppMode) -> Text<'_> {
        if !self.command_error.is_empty() {
            return Text::from_iter(self.command_error.lines().map(|l| l.to_string().red()));
        }

        if app_mode != AppMode::Command {
            return Text::default();
        }

        let mut text = Text::styled(
            format!(":{}", self.current_command),
            (Color::White, Modifier::BOLD),
        );

        text.push_span(Span::styled("█", Modifier::SLOW_BLINK));

        let completions = self.command_bar_completions();
        if let Some(completions_line) = completions {
            text.push_line(completions_line);
        }

        text
    }

    pub fn read_event(&mut self, event: Event) -> (Option<AppMode>, Option<Command>) {
        if !self.command_error.is_empty() {
            self.exit_command_mode();
            return (Some(AppMode::Logs), None);
        }

        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.current_command.push(c);
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::SHIFT,
                ..
            }) => {
                self.current_command.push(c.to_ascii_uppercase());
            }
            Event::Key(KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                let mut curr_cmd = String::new();
                std::mem::swap(&mut curr_cmd, &mut self.current_command);

                match curr_cmd.parse() {
                    Ok(cmd) => {
                        self.exit_command_mode();
                        return (Some(AppMode::Logs), Some(cmd));
                    }
                    Err(err) => {
                        // Stay in command mode for one more keystroke
                        self.command_error = format!("{err}");
                        return (None, None);
                    }
                }
            }
            Event::Key(KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.current_command.pop();
            }
            Event::Key(KeyEvent {
                code: KeyCode::Esc, ..
            }) => {
                self.exit_command_mode();
                return (Some(AppMode::Logs), None);
            }
            Event::Key(KeyEvent {
                code: KeyCode::Tab,
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                let (longest_common_prefix, completions) =
                    Command::auto_complete(&self.current_command);

                if let Some(prefix) = longest_common_prefix {
                    self.current_command = prefix;
                }

                if completions.len() > 1 {
                    self.command_completions = completions;
                }
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('w'),
                modifiers: KeyModifiers::CONTROL,
                ..
            }) => {
                if let Some(space) = self.current_command.rfind(' ') {
                    self.current_command.truncate(space);
                } else {
                    self.current_command.clear();
                }
            }
            _ => (),
        }

        (None, None)
    }
}
