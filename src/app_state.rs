use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind};

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
        }
    }

    pub fn total_filters_enabled(&self) -> usize {
        self.filter_ins.len() + self.filter_outs.len()
    }

    pub fn status_bar_text(&self) -> String {
        if self.command_mode {
            format!(":{}", self.current_command)
        } else {
            String::new()
        }
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

    fn handle_command(&mut self) -> Option<()> {
        let mut curr_cmd = String::new();
        std::mem::swap(&mut curr_cmd, &mut self.current_command);
        let (command, arguments) = curr_cmd.trim().split_once(|c: char| c.is_whitespace())?;
        match command {
            "filter-in" => {
                let r = regex::Regex::new(arguments).ok()?;
                self.filter_ins.push(r);
            }
            "filter-out" => {
                let r = regex::Regex::new(arguments).ok()?;
                self.filter_outs.push(r);
            }
            _ => return None,
        }

        Some(())
    }

    pub fn read_event(&mut self, event: Event) -> AppAction {
        let state = self;
        if state.command_mode {
            match event {
                Event::Key(KeyEvent {
                    code: KeyCode::Char(c),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => {
                    state.current_command.push(c);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char(c),
                    modifiers: KeyModifiers::SHIFT,
                    ..
                }) => {
                    state.current_command.push(c.to_ascii_uppercase());
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => {
                    state.command_mode = false;
                    // Ignore bad commands for now
                    let _ = state.handle_command();
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Backspace,
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => {
                    state.current_command.pop();
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc, ..
                }) => {
                    state.command_mode = false;
                    state.current_command.clear();
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
                    state.offset = match key.code {
                        // KeyCode::Left | KeyCode::Char('h') => app.on_left(),
                        KeyCode::Up | KeyCode::Char('k') => state.offset.saturating_sub(1),
                        // KeyCode::Right | KeyCode::Char('l') => app.on_right(),
                        KeyCode::Down | KeyCode::Char('j') => state.offset.saturating_add(1),
                        // KeyCode::Char(c) => app.on_key(c),
                        _ => state.offset,
                    };

                    state.offset = state.offset.min(state.lines.len() - 1);

                    match key.code {
                        KeyCode::Char(':') => {
                            state.command_mode = true;
                        }
                        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            state.filter_ins.clear();
                            state.filter_outs.clear();
                        }
                        _ => (),
                    }
                }
            }
            Event::Mouse(mouse) => {
                state.offset = match mouse.kind {
                    MouseEventKind::ScrollDown => state.offset.saturating_sub(1),
                    MouseEventKind::ScrollUp => state.offset.saturating_add(1),
                    _ => state.offset,
                };

                state.offset = state.offset.min(state.lines.len() - 1);
            }
            _ => (),
        }

        AppAction::NoAction
    }
}
