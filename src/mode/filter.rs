use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{
    layout::Constraint,
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Row, Table, TableState},
};
use regex::Regex;

use crate::app_state::{AppAction, AppMode};

const MAX_FILTERS_TABLE_ROWS_DISPLAYED: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    In,
    Out,
}

pub struct Filter {
    kind: FilterKind,
    regex: regex::Regex,
    enabled: bool,
}

impl Filter {
    pub fn new(kind: FilterKind, regex: regex::Regex) -> Self {
        Self {
            kind,
            regex,
            enabled: true,
        }
    }

    pub fn keep_line(&self, line: &str) -> bool {
        if !self.enabled {
            return true;
        }

        match self.kind {
            FilterKind::In => self.regex.is_match(line),
            FilterKind::Out => !self.regex.is_match(line),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn filter_kind(&self) -> FilterKind {
        self.kind
    }

    pub fn regex(&self) -> &str {
        self.regex.as_str()
    }
}

pub struct Filters {
    filters: Vec<Filter>,
    selected_filter: usize,
    filtering_enabled: bool,
}

impl Filters {
    pub fn new() -> Self {
        Self {
            filters: vec![],
            selected_filter: 0,
            filtering_enabled: true,
        }
    }

    pub fn clear(&mut self) {
        self.filters.clear();
        self.selected_filter = 0;
    }

    pub fn create_in_filter(&mut self, r: Regex) {
        self.filters.push(Filter::new(FilterKind::In, r));
    }

    pub fn create_out_filter(&mut self, r: Regex) {
        self.filters.push(Filter::new(FilterKind::Out, r));
    }

    pub fn keep_line(&self, line: &str) -> bool {
        if self.filters.is_empty() || !self.filtering_enabled {
            return true;
        }

        let filtered_in = self
            .filters
            .iter()
            .filter(|f| f.filter_kind() == FilterKind::In)
            .fold(None, |acc, f| {
                Some(acc.unwrap_or(false) || f.keep_line(line))
            })
            .unwrap_or(true);

        let filtered_out = self
            .filters
            .iter()
            .filter(|f| f.filter_kind() == FilterKind::Out)
            .any(|f| !f.keep_line(line));

        filtered_in && !filtered_out
    }

    pub fn total_filters_enabled(&self) -> usize {
        self.filters.iter().filter(|f| f.is_enabled()).count()
    }

    pub fn filters_menu_info_lines_size(&self) -> usize {
        if self.filters.is_empty() { 1 } else { 2 }
    }

    pub fn filters_menu_size(&self) -> usize {
        // Don't display all filters if there are too many
        self.filters.len().min(MAX_FILTERS_TABLE_ROWS_DISPLAYED)
            + self.filters_menu_info_lines_size()
    }

    fn filters_menu_info_lines(&self) -> Text<'_> {
        let total_filters = self.filters.len();
        let enabled_filters = self.total_filters_enabled();

        let filtering_disabled_text = if self.filtering_enabled {
            ""
        } else {
            " (Filtering disabled)"
        };

        let enabled = Line::styled(
            format!(
                "Text filters: {enabled_filters} of {total_filters} enabled {filtering_disabled_text}",
            ),
            (Color::White, Color::DarkGray),
        );

        if total_filters == 0 {
            return Text::from(enabled);
        }

        let filter = &self.filters[self.selected_filter];

        let space_does = if filter.is_enabled() {
            "disable"
        } else {
            "enable"
        };

        let t_does = match filter.filter_kind() {
            FilterKind::In => "OUT",
            FilterKind::Out => "IN",
        };

        let f_does = if self.filtering_enabled {
            "Disable"
        } else {
            "Enable"
        };

        let keys = Line::styled(
            format!("SPC: {space_does}   t: To {t_does}   D: Delete   f: {f_does} Filtering"),
            (Color::White, Color::DarkGray),
        );

        Text::from_iter([enabled, keys])
    }

    pub fn filters_menu_text(&self) -> (Text<'_>, Table<'_>, TableState) {
        let rows = self.filters.iter().map(|filter| {
            let enabled = if filter.is_enabled() {
                Span::styled(" ◆", Color::Green)
            } else {
                Span::styled("  ", Color::Red)
            };

            let kind = match filter.filter_kind() {
                FilterKind::In => Span::styled("IN", Color::Green).bold(),
                FilterKind::Out => Span::styled("OUT", Color::Red).bold(),
            };

            let regex = Span::styled(format!(" | {}", filter.regex()), Color::White);

            Row::from_iter([enabled, kind, regex])
        });

        let mut table = Table::new(
            rows,
            [Constraint::Max(3), Constraint::Max(5), Constraint::Min(70)],
        );

        table = table.row_highlight_style(Style::default().bg(Color::DarkGray));
        table = table.highlight_symbol("→ ");

        let table_state = TableState::new()
            .with_selected(Some(self.selected_filter))
            .with_offset(self.selected_filter);

        let info_lines = self.filters_menu_info_lines();

        (info_lines, table, table_state)
    }

    pub fn read_event(&mut self, event: Event) -> (Option<AppMode>, Option<AppAction>) {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Tab, ..
            }) => return (Some(AppMode::Logs), None),
            Event::Key(KeyEvent {
                code: KeyCode::Up | KeyCode::Char('k'),
                ..
            }) => {
                self.selected_filter = self.selected_filter.saturating_sub(1);
            }
            Event::Key(KeyEvent {
                code: KeyCode::Down | KeyCode::Char('j'),
                ..
            }) => {
                self.selected_filter = self
                    .selected_filter
                    .saturating_add(1)
                    .min(self.filters.len().saturating_sub(1));
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char(' '),
                ..
            }) => {
                if !self.filters.is_empty() {
                    let filter = &mut self.filters[self.selected_filter];
                    filter.enabled = !filter.enabled;
                }
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('t'),
                ..
            }) => {
                if !self.filters.is_empty() {
                    let filter = &mut self.filters[self.selected_filter];
                    filter.kind = match filter.filter_kind() {
                        FilterKind::In => FilterKind::Out,
                        FilterKind::Out => FilterKind::In,
                    };
                }
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('D'),
                ..
            }) => {
                if !self.filters.is_empty() {
                    self.filters.remove(self.selected_filter);
                    self.selected_filter = self.selected_filter.saturating_sub(1);
                }
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('f'),
                ..
            }) => {
                self.filtering_enabled = !self.filtering_enabled;
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                ..
            }) => return (Some(AppMode::Logs), None),
            _ => (),
        }

        (None, None)
    }
}
