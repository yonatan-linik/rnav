use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::style::{Color, Modifier, Stylize as _};
use ratatui::text::{Span, Text};

use crate::app_state::{AppAction, AppMode};

pub struct Search {
    past_patterns: Vec<String>,
    last_selected_pattern: Option<usize>,
    pattern: String,
    active_pattern: Option<regex::Regex>,
    search_error: String,
}

impl Search {
    pub fn new() -> Self {
        Self {
            past_patterns: Vec::new(),
            last_selected_pattern: None,
            pattern: String::new(),
            active_pattern: None,
            search_error: String::new(),
        }
    }

    pub fn read_event(&mut self, event: Event) -> (Option<AppMode>, Option<AppAction>) {
        let Event::Key(KeyEvent { code: key, .. }) = event else {
            return (None, None);
        };

        if !self.search_error.is_empty() {
            self.search_error.clear();
            return (Some(AppMode::Logs), None);
        }

        match key {
            KeyCode::Char('\n') | KeyCode::Enter => {
                let regex = match regex::Regex::new(self.pattern.as_str()) {
                    Err(e) => {
                        self.active_pattern = None;
                        self.search_error = format!("{e}");
                        return (Some(AppMode::Search), None);
                    }
                    Ok(regex) => regex,
                };

                self.active_pattern = Some(regex);
                self.past_patterns.push(self.pattern.clone());
                self.pattern.clear();
                (Some(AppMode::Logs), None)
            }
            KeyCode::Esc => {
                self.last_selected_pattern = None;
                if self.pattern.is_empty() {
                    (Some(AppMode::Logs), None)
                } else {
                    self.pattern.clear();
                    (Some(AppMode::Search), None)
                }
            }
            KeyCode::Char(c) => {
                self.pattern.push(c);

                (None, None)
            }
            KeyCode::Backspace => {
                if self.pattern.is_empty() {
                    (Some(AppMode::Logs), None)
                } else {
                    self.pattern.pop();
                    (None, None)
                }
            }
            KeyCode::Up => {
                let index = self
                    .last_selected_pattern
                    .get_or_insert(self.past_patterns.len());

                *index = (*index).saturating_sub(1);
                if let Some(pattern) = self.past_patterns.get(*index) {
                    self.pattern = pattern.clone();
                }

                (None, None)
            }
            KeyCode::Down => {
                let Some(index) = self.last_selected_pattern.as_mut() else {
                    return (None, None);
                };

                *index = (*index).saturating_add(1);
                if let Some(pattern) = self.past_patterns.get(*index) {
                    self.pattern = pattern.clone();
                }

                (None, None)
            }
            _ => (None, None),
        }
    }

    pub fn command_bar_text(&self, app_mode: &AppMode) -> ratatui::prelude::Text<'_> {
        if !self.search_error.is_empty() {
            return Text::from_iter(self.search_error.lines().map(|l| l.to_string().red()));
        }

        if !matches!(app_mode, AppMode::Search) {
            return Text::default();
        }

        let mut text = Text::styled(
            format!("/{}", self.pattern),
            (Color::White, Modifier::empty()),
        );

        text.push_span(Span::styled("█", Modifier::SLOW_BLINK));

        text
    }

    pub fn active_search_pattern(&self) -> Option<&regex::Regex> {
        self.active_pattern.as_ref()
    }

    pub fn clear(&mut self) {
        self.pattern.clear();
        self.last_selected_pattern = None;
        self.active_pattern = None;
        self.search_error.clear();
        self.past_patterns.clear();
    }
}
