use crate::app_state::AppMode;
use crate::error::{Error, Result};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Color, Modifier, Stylize as _};
use ratatui::text::{Line, Span, Text};
use strum::EnumIter;
use strum::IntoEnumIterator;

#[derive(Debug, EnumIter, Copy, Clone)]
pub enum CommandType {
    FilterIn,
    FilterOut,
    Highlight,
}

impl std::str::FromStr for CommandType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "filter-in" => Ok(CommandType::FilterIn),
            "filter-out" => Ok(CommandType::FilterOut),
            "highlight" => Ok(CommandType::Highlight),
            _ => Err(Error::UnknownCommand(s.to_string())),
        }
    }
}

impl From<CommandType> for &'static str {
    fn from(command: CommandType) -> Self {
        match command {
            CommandType::FilterIn => "filter-in",
            CommandType::FilterOut => "filter-out",
            CommandType::Highlight => "highlight",
        }
    }
}

impl CommandType {
    fn as_str(&self) -> &'static str {
        (*self).into()
    }

    pub fn auto_complete(prefix: &str) -> (Option<String>, Vec<&'static str>) {
        let completions: Vec<&'static str> = CommandType::iter()
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

pub struct Command {
    pub cmd_type: CommandType,
    pub args: String,
}

impl std::str::FromStr for Command {
    type Err = Error;

    fn from_str(curr_cmd: &str) -> Result<Self> {
        let (command, args) = curr_cmd
            .trim()
            .split_once(|c: char| c.is_whitespace())
            .unwrap_or((curr_cmd.trim(), ""));

        let cmd_type: CommandType = command.parse()?;

        // Currently all commands need arguments
        if args.is_empty() {
            return Err(Error::NoArgumentsGivenToCommand);
        }

        Ok(Command {
            cmd_type,
            args: args.to_string(),
        })
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

    pub fn set_command_error(&mut self, err: impl Into<String>) {
        self.command_error = err.into();
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
                    CommandType::auto_complete(&self.current_command);

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
