use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind};
use crossterm::style::{StyledContent, Stylize};

use crate::error::{Error, Result};
use crate::log_line::LogLine;

pub enum AppAction {
    EndApp,
    NoAction,
}

pub struct AppState<'a> {
    file_name: String,
    lines: Vec<LogLine<'a>>,
    offset: usize,
    filter_ins: Vec<regex::Regex>,
    filter_outs: Vec<regex::Regex>,
    command_mode: bool,
    current_command: String,
    present_error: String,
}

impl<'a> AppState<'a> {
    pub fn new(text: &'a str, file_name: impl Into<String>) -> Self {
        let mut lines: Vec<_> = text
            .lines()
            .map(|l| {
                LogLine::new(l).unwrap_or_else(|| LogLine {
                    time: chrono::DateTime::<chrono::Utc>::MAX_UTC.fixed_offset(),
                    log: l,
                })
            })
            .collect();
        lines.sort_by(|a, b| a.time.cmp(&b.time));

        Self {
            file_name: file_name.into(),
            lines,
            offset: 0,
            filter_ins: Vec::new(),
            filter_outs: Vec::new(),
            command_mode: false,
            current_command: String::new(),
            present_error: String::new(),
        }
    }

    pub fn total_filters_enabled(&self) -> usize {
        self.filter_ins.len() + self.filter_outs.len()
    }

    pub fn status_bar_text(&self) -> StyledContent<String> {
        if !self.present_error.is_empty() {
            return self.present_error.clone().red();
        }

        if self.command_mode {
            format!(":{}", self.current_command)
        } else {
            String::new()
        }
        .with(crossterm::style::Color::Rgb {
            r: 255,
            g: 255,
            b: 255,
        })
        .bold()
    }

    pub fn state_bar_text_number_of_lines(&self) -> usize {
        self.status_bar_text().content().lines().count()
    }

    pub fn main_area_title(&self) -> &str {
        &self.file_name
    }

    pub fn lines_iter(&self) -> impl IntoIterator<Item = LogLine> {
        self.lines
            .iter()
            .copied()
            .skip(self.offset)
            .filter(|l| {
                self.filter_ins.is_empty() || self.filter_ins.iter().any(|f| f.is_match(l.log))
            })
            .filter(|l| {
                self.filter_outs.is_empty() || self.filter_outs.iter().all(|f| !f.is_match(l.log))
            })
    }

    fn handle_command(&mut self) -> Result<()> {
        let mut curr_cmd = String::new();
        std::mem::swap(&mut curr_cmd, &mut self.current_command);
        let (command, arguments) = curr_cmd
            .trim()
            .split_once(|c: char| c.is_whitespace())
            .ok_or(Error::NoArgumentsGivenToCommand)?;
        match command {
            "filter-in" => {
                let r = regex::Regex::new(arguments)?;
                self.filter_ins.push(r);
            }
            "filter-out" => {
                let r = regex::Regex::new(arguments)?;
                self.filter_outs.push(r);
            }
            _ => return Err(Error::UnknownCommand(command.to_string())),
        }

        Ok(())
    }

    pub fn read_event(&mut self, event: Event) -> AppAction {
        if matches!(event, Event::Resize(_, _)) {
            return AppAction::NoAction;
        }

        self.present_error = String::new();
        if self.command_mode {
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
                    self.command_mode = false;
                    if let Err(err) = self.handle_command() {
                        self.present_error = format!("{err}");
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
                    self.command_mode = false;
                    self.current_command.clear();
                }
                _ => (),
            }
            return AppAction::NoAction;
        }

        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Esc, ..
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                return AppAction::EndApp;
            }
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    self.offset = match key.code {
                        // KeyCode::Left | KeyCode::Char('h') => app.on_left(),
                        KeyCode::Up | KeyCode::Char('k') => self.offset.saturating_sub(1),
                        // KeyCode::Right | KeyCode::Char('l') => app.on_right(),
                        KeyCode::Down | KeyCode::Char('j') => self.offset.saturating_add(1),
                        // KeyCode::Char(c) => app.on_key(c),
                        _ => self.offset,
                    };

                    self.offset = self.offset.min(self.lines.len() - 1);

                    match key.code {
                        KeyCode::Char(':') => {
                            self.command_mode = true;
                        }
                        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.filter_ins.clear();
                            self.filter_outs.clear();
                        }
                        _ => (),
                    }
                }
            }
            Event::Mouse(mouse) => {
                self.offset = match mouse.kind {
                    MouseEventKind::ScrollDown => self.offset.saturating_sub(1),
                    MouseEventKind::ScrollUp => self.offset.saturating_add(1),
                    _ => self.offset,
                };

                self.offset = self.offset.min(self.lines.len() - 1);
            }
            _ => (),
        }

        AppAction::NoAction
    }
}
