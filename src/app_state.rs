use std::iter::once;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind};
use crossterm::style::{StyledContent, Stylize};
use rand::Rng as _;
use ratatui::style::Color;
use ratatui::text::{Line, Span};

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
    highlights: Vec<(regex::Regex, Color)>,
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
            filter_ins: vec![],
            filter_outs: vec![],
            command_mode: false,
            current_command: String::new(),
            present_error: String::new(),
            highlights: vec![],
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

    fn apply_filter_ins(&self, l: &LogLine) -> bool {
        self.filter_ins.is_empty() || self.filter_ins.iter().any(|f| f.is_match(l.log))
    }

    fn apply_filter_outs(&self, l: &LogLine) -> bool {
        self.filter_outs.is_empty() || self.filter_outs.iter().all(|f| !f.is_match(l.log))
    }

    fn split_keep<'b>(
        r: &regex::Regex,
        text: &'b str,
        base_color: Color,
        color: Color,
    ) -> impl IntoIterator<Item = Span<'b>> {
        let mut result = Vec::new();
        let mut last = 0;
        for m in r.find_iter(text) {
            if last != m.start() {
                result.push(Span::styled(&text[last..m.start()], base_color));
            }
            result.push(Span::styled(m.as_str(), color));
            last = m.end();
        }
        if last < text.len() {
            result.push(Span::styled(&text[last..], base_color));
        }
        result
    }

    fn apply_highlights_to_line(&'a self, log_line: LogLine<'a>) -> Line<'a> {
        let mut spans: Box<dyn Iterator<Item = Span<'a>>> = Box::new(once(Span::raw(log_line.log)));

        for (regex, color) in &self.highlights {
            let new_spans: Box<dyn Iterator<Item = Span<'a>>> = Box::new(
                spans
                    .flat_map(|s| {
                        let b = match s.content {
                            std::borrow::Cow::Borrowed(b) => b,
                            std::borrow::Cow::Owned(_) => unreachable!("This can never be owned, it is always borrowed from the original log text, and we don't modify it"),
                        };

                        AppState::split_keep(regex, b, s.style.fg.unwrap_or_default(), *color)
                    }),
            );

            spans = new_spans;
        }

        Line::from_iter(spans)
    }

    fn apply_highlights(
        &'a self,
        log_lines: impl IntoIterator<Item = LogLine<'a>> + 'a,
    ) -> impl IntoIterator<Item = Line<'a>> + 'a {
        log_lines
            .into_iter()
            .map(|l| self.apply_highlights_to_line(l))
    }

    pub fn lines_iter(&'a self) -> impl IntoIterator<Item = Line<'a>> + 'a {
        let filtered_lines = self
            .lines
            .iter()
            .copied()
            .skip(self.offset)
            .filter(|l| self.apply_filter_ins(l))
            .filter(|l| self.apply_filter_outs(l));

        self.apply_highlights(filtered_lines)
    }

    fn handle_command(&mut self) -> Result<()> {
        const COMMANDS: &[&str] = &["filter-in", "filter-out", "highlight"];
        let mut curr_cmd = String::new();
        std::mem::swap(&mut curr_cmd, &mut self.current_command);
        let (command, arguments) = curr_cmd
            .trim()
            .split_once(|c: char| c.is_whitespace())
            .unwrap_or((curr_cmd.trim(), ""));

        if COMMANDS.contains(&command) && arguments.is_empty() {
            return Err(Error::NoArgumentsGivenToCommand);
        }

        match command {
            "filter-in" => {
                let r = regex::Regex::new(arguments)?;
                self.filter_ins.push(r);
            }
            "filter-out" => {
                let r = regex::Regex::new(arguments)?;
                self.filter_outs.push(r);
            }
            "highlight" => {
                let r = regex::Regex::new(arguments)?;
                self.highlights.push((
                    r,
                    Color::from_u32(rand::thread_rng().gen_range(255..=0x00FF_FFFF)),
                ));
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
                            self.highlights.clear();
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
