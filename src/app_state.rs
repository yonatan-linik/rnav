use std::cmp::Ordering;
use std::iter::once;
use std::sync::Arc;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind};
use itertools::Itertools;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span, Text};

use crate::log::log_file::LogFile;
use crate::log::log_line::LogLine;
use crate::mode::command::{Command, Commands};
use crate::mode::filter::Filters;

pub enum AppAction {
    EndApp,
    NoAction,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Logs,
    Command,
    FiltersMenu,
}

pub struct AppState<'a> {
    file_names: Vec<Arc<str>>,
    lines: Vec<LogLine<'a>>,
    line_offset: usize,
    column_offset: usize,
    pub filters: Filters,
    commands: Commands,
    mode: AppMode,
    highlights: Vec<(regex::Regex, Color)>,
    show_file_names: bool,
    word_wrapping: bool,
}

impl<'a> AppState<'a> {
    pub fn new(files: &'a [LogFile]) -> Self {
        let mut lines: Vec<_> = files
            .iter()
            .flat_map(|src_file| {
                src_file
                    .contents()
                    .split(|c| *c == b'\n')
                    .map(|l| String::from_utf8_lossy(l))
                    .filter_map(|l| (!l.trim().is_empty()).then(|| LogLine::new(src_file, l)))
            })
            .collect();
        // Keep the lines order inside the same file, only order by time between files
        lines.sort_by(|a, b| match a.src_file.name.cmp(&b.src_file.name) {
            Ordering::Equal => Ordering::Equal,
            _ => a.time.cmp(&b.time),
        });

        let file_names = files.iter().map(|f| f.name.clone()).collect();

        Self {
            lines,
            line_offset: 0,
            column_offset: 0,
            filters: Filters::new(),
            mode: AppMode::Logs,
            highlights: vec![],
            show_file_names: false,
            file_names,
            commands: Commands::new(),
            word_wrapping: false,
        }
    }

    pub fn mode(&self) -> AppMode {
        self.mode
    }

    fn reset_session(&mut self) {
        self.filters.clear();
        self.highlights.clear();
        self.lines.iter_mut().for_each(|line| {
            line.marked = false;
            line.comment = None;
        });
    }

    pub fn state_bar_text_number_of_lines(&self) -> usize {
        match self.mode {
            AppMode::Command => self.commands.command_bar_text(self.mode).lines.len(),
            AppMode::Logs => 0,
            AppMode::FiltersMenu => self.filters.filters_menu_size(),
        }
    }

    pub fn command_bar_text(&self) -> Text<'_> {
        self.commands.command_bar_text(self.mode)
    }

    pub fn get_line_offset(&self) -> usize {
        self.line_offset
    }

    fn split_keep<'b>(
        r: &regex::Regex,
        text: &'b str,
        base_style: Style,
        style: Style,
    ) -> impl IntoIterator<Item = Span<'b>> {
        let mut result = Vec::new();
        let mut last = 0;
        for m in r.find_iter(text) {
            if last != m.start() {
                result.push(Span::styled(&text[last..m.start()], base_style));
            }
            result.push(Span::styled(m.as_str(), style));
            last = m.end();
        }
        if last < text.len() {
            result.push(Span::styled(&text[last..], base_style));
        }
        result
    }

    fn apply_highlights_to_line(&'a self, span: Span<'a>) -> Line<'a> {
        // If the log line is marked make sure it is for the entire line
        let bg_color = span.style.bg.unwrap_or_default();
        let mut spans: Box<dyn Iterator<Item = Span<'a>>> = Box::new(once(span));

        for (regex, color) in &self.highlights {
            let new_spans: Box<dyn Iterator<Item = Span<'a>>> = Box::new(
                spans
                    .flat_map(|s| {
                        let b = match s.content {
                            std::borrow::Cow::Borrowed(b) => b,
                            std::borrow::Cow::Owned(_) => unreachable!("This can never be owned, it is always borrowed from the original log text, and we don't modify it"),
                        };

                        AppState::split_keep(regex, b, s.style, s.style.fg(*color))
                    }),
            );

            spans = new_spans;
        }

        Line::from_iter(spans).bg(bg_color)
    }

    fn apply_highlights(
        &'a self,
        log_lines: impl IntoIterator<Item = Span<'a>> + 'a,
    ) -> impl IntoIterator<Item = Line<'a>> + 'a {
        log_lines
            .into_iter()
            .map(|s| self.apply_highlights_to_line(s))
    }

    fn filter_all_lines_iter(&'a self) -> impl IntoIterator<Item = (usize, &'a LogLine<'a>)> {
        self.lines
            .iter()
            .enumerate()
            .filter(|(_, l)| self.filters.keep_line(&l.log))
    }

    fn filter_lines_iter(&'a self) -> impl IntoIterator<Item = (usize, &'a LogLine<'a>)> {
        self.filter_all_lines_iter()
            .into_iter()
            .skip(self.line_offset)
    }

    pub fn filtered_lines_count(&self) -> usize {
        self.filter_all_lines_iter().into_iter().count()
    }

    fn apply_marks_and_offset(
        &'a self,
        log_lines: impl IntoIterator<Item = &'a LogLine<'a>> + 'a,
    ) -> impl IntoIterator<Item = Span<'a>> + 'a {
        log_lines.into_iter().map(|l| {
            let offset = self.column_offset.min(l.log.len());
            let log = &l.log[offset..];
            if l.marked {
                Span::styled(log, Style::new().bg(Color::White).fg(Color::Black))
            } else {
                Span::raw(log)
            }
        })
    }

    fn filtered_lines_file_names(&'a self) -> impl IntoIterator<Item = Span<'a>> + 'a {
        let lines = self
            .filter_lines_iter()
            .into_iter()
            .map(|(_, l)| l.src_file);

        let max_file_name_length = self.file_names.iter().map(|n| n.len()).max().unwrap_or(0);

        once(None)
            .chain(lines.into_iter().map(Some))
            .chain(once(None))
            .tuple_windows()
            .map(move |(prev, curr, next)| {
                let curr = curr.expect("curr will always be Some()");
                let sep = match (&prev, &next) {
                    (None, None) => "[",
                    (None, Some(n)) if curr.name == n.name => "┌",
                    // If the next line isn't from the same file
                    (None, _) => "[",
                    // If the previous line isn't from the same file
                    (Some(p), None) if curr.name == p.name => "└",
                    (_, None) => "[",
                    (Some(p), Some(n)) if curr.name == p.name && curr.name == n.name => "├",
                    (Some(p), Some(n)) if curr.name == p.name && curr.name != n.name => "└",
                    (Some(p), Some(n)) if curr.name != p.name && curr.name == n.name => "┌",
                    // If both are different from current
                    (Some(_), Some(_)) => "[",
                };

                if !self.show_file_names {
                    Span::styled(sep, (curr.color, Color::default()))
                } else {
                    Span::styled(
                        format!("{:width$}{sep}", curr.name, width = max_file_name_length),
                        (curr.color, Color::default()),
                    )
                }
            })
    }

    fn render_comment_line(&'a self, comment: &'a str, max_file_name_length: usize) -> Line<'a> {
        // Build the file-name-column padding for comment lines so comments do not influence
        // the range marker logic. Keep the prefix '└ ' unstyled and only style the comment content.
        // The comment background must NOT inherit mark backgrounds; always use the default bg here.
        let file_span_comment = if !self.show_file_names {
            // single-space placeholder when file names are hidden
            Span::raw(" ")
        } else {
            // padded empty column matching file name width
            Span::raw(format!("{:width$}", "", width = max_file_name_length))
        };

        let comment_prefix = Span::raw("└ ");
        let comment_span = Span::styled(comment, Color::Green);

        Line::from_iter(
            once(file_span_comment)
                .chain(once(comment_prefix))
                .chain(once(comment_span)),
        )
        .bg(Color::default())
    }

    pub fn lines_iter(&'a self) -> impl IntoIterator<Item = Line<'a>> + 'a {
        // Keep everything lazy: create two clones of the filtered log lines stream.
        let filtered_for_render = self.filter_lines_iter().into_iter().map(|(_, l)| l);

        let filtered_for_comments = self
            .filter_lines_iter()
            .into_iter()
            .map(|(_, l)| &l.comment);

        let marked_lines = self.apply_marks_and_offset(filtered_for_render);
        let highlighted_lines = self.apply_highlights(marked_lines);
        let named_lines = self.filtered_lines_file_names();

        // Precompute file name width for comment padding
        let max_file_name_length = self.file_names.iter().map(|n| n.len()).max().unwrap_or(0);

        // Zip name, highlighted line and comment (option) together and lazily emit one or two visual lines.
        named_lines
            .into_iter()
            .zip(highlighted_lines)
            .zip(filtered_for_comments)
            .flat_map(move |((file_name, highlighted_line), comment_opt)| {
                // main line
                let log_line = Line::from_iter(once(file_name).chain(highlighted_line.spans))
                    .bg(highlighted_line.style.bg.unwrap_or_default());

                // optional comment line (do not affect range markers; use padding in file column)
                once(log_line).chain(
                    comment_opt
                        .as_ref()
                        .map(|c| self.render_comment_line(c, max_file_name_length)),
                )
            })
    }

    pub fn top_log_line_title_bar_text(&self) -> Line<'_> {
        let Some(l) = self.filter_lines_iter().into_iter().next().map(|(_, l)| l) else {
            return Line::default();
        };

        let log_text = Span::styled(" LOG ", (Color::White, l.src_file.color));

        let log_time = if l.time == chrono::DateTime::<chrono::Utc>::MAX_UTC.fixed_offset() {
            Span::raw("⟩Unknown time⟩")
        } else {
            Span::raw(format!("⟩{}⟩", l.time.to_rfc3339()))
        };

        let log_file_name = Span::raw(l.src_file.name.replace('/', "⟩"));

        let bg_color = Color::Rgb(40, 40, 40);

        let log_level_title = Span::raw(" Level:");
        let log_level = l.level.into();

        Line::from_iter([
            log_text,
            log_level_title,
            log_level,
            log_time,
            log_file_name,
        ])
        .fg(Color::Gray)
        .bg(bg_color)
    }

    fn handle_command(&mut self, command: Command) {
        match command {
            Command::FilterIn(r) => {
                self.filters.create_in_filter(r);
            }
            Command::FilterOut(r) => {
                self.filters.create_out_filter(r);
            }
            Command::Highlight(r) => {
                self.highlights
                    .push((r, Color::from_u32(rand::random_range(255..=0x00FF_FFFF))));
            }
            Command::ToggleWrapping => self.word_wrapping = !self.word_wrapping,
            Command::Comment(text) => {
                if let Some(i) = self.top_log_line_index() {
                    self.lines[i].comment = Some(text);
                }
            }
            Command::ClearComment => {
                if let Some(i) = self.top_log_line_index() {
                    self.lines[i].comment = None;
                }
            }
        }
    }

    fn longest_filtered_log(&self) -> usize {
        self.filter_lines_iter()
            .into_iter()
            .map(|(_, l)| l.log.len())
            .max()
            .unwrap_or(0)
    }

    pub fn read_event(
        &mut self,
        event: Event,
        terminal: &ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    ) -> AppAction {
        if matches!(event, Event::Resize(_, _)) {
            return AppAction::NoAction;
        }

        if let Event::Key(KeyEvent {
            kind: KeyEventKind::Release,
            ..
        }) = event
        {
            return AppAction::NoAction;
        }

        let shown_lines_count = self.filtered_lines_count();
        self.line_offset = self.line_offset.min(shown_lines_count - 1);

        match self.mode {
            AppMode::Command => {
                let (new_mode, cmd) = self.commands.read_event(event);
                self.mode = new_mode.unwrap_or(AppMode::Command);
                if let Some(cmd) = cmd {
                    self.handle_command(cmd);
                }
            }
            AppMode::FiltersMenu => {
                let (new_mode, app_action) = self.filters.read_event(event);
                self.mode = new_mode.unwrap_or(AppMode::FiltersMenu);
                return app_action.unwrap_or(AppAction::NoAction);
            }
            AppMode::Logs => {
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
                            match key.code {
                                KeyCode::Left | KeyCode::Char('h') => {
                                    if self.column_offset == 0 {
                                        self.show_file_names = true
                                    }
                                    self.column_offset = self.column_offset.saturating_sub(10)
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    self.line_offset = self.line_offset.saturating_sub(1)
                                }
                                KeyCode::Right | KeyCode::Char('l') => {
                                    if !self.show_file_names {
                                        self.column_offset =
                                            self.column_offset.saturating_add(10).min(
                                                self.longest_filtered_log().saturating_sub(1) / 10
                                                    * 10,
                                            );
                                    } else {
                                        self.show_file_names = false;
                                    }
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    self.line_offset = self.line_offset.saturating_add(1)
                                }
                                KeyCode::Char('g') => {
                                    self.line_offset = 0;
                                }
                                KeyCode::Char('G') => {
                                    self.line_offset =
                                        self.filtered_lines_count().saturating_sub(1);
                                }
                                KeyCode::PageDown | KeyCode::PageUp => {
                                    let page_size = terminal
                                        .size()
                                        .expect("Should be able to get terminal size")
                                        .height
                                        // 2 Lines for header and footer + 1 so you have an overlapping line from previous page
                                        .saturating_sub(3)
                                        as usize;

                                    self.line_offset = if KeyCode::PageDown == key.code {
                                        self.line_offset.saturating_add(page_size)
                                    } else {
                                        self.line_offset.saturating_sub(page_size)
                                    };
                                }
                                // KeyCode::Char(c) => app.on_key(c),
                                _ => (),
                            };

                            self.line_offset = self
                                .line_offset
                                .min(self.filtered_lines_count().saturating_sub(1));

                            match key.code {
                                KeyCode::Char(':') => {
                                    self.mode = AppMode::Command;
                                }
                                KeyCode::Char('r')
                                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
                                    self.reset_session();
                                }
                                KeyCode::Char('m') => {
                                    self.flip_mark_of_top_log_line();
                                }
                                KeyCode::Char('U') => {
                                    self.goto_prev_mark();
                                }
                                KeyCode::Char('u') => {
                                    self.goto_next_mark();
                                }
                                KeyCode::Tab => {
                                    self.mode = AppMode::FiltersMenu;
                                }
                                _ => (),
                            }
                        }
                    }
                    Event::Mouse(mouse) => {
                        self.line_offset = match mouse.kind {
                            MouseEventKind::ScrollDown => self.line_offset.saturating_sub(1),
                            MouseEventKind::ScrollUp => self.line_offset.saturating_add(1),
                            _ => self.line_offset,
                        };

                        self.line_offset = self.line_offset.min(self.lines.len() - 1);
                    }
                    _ => (),
                }
            }
        }

        AppAction::NoAction
    }

    fn top_log_line_index(&self) -> Option<usize> {
        self.filter_lines_iter().into_iter().next().map(|(i, _)| i)
    }

    fn flip_mark_of_top_log_line(&mut self) {
        let Some(i) = self.top_log_line_index() else {
            return;
        };

        self.lines[i].marked = !self.lines[i].marked;
    }

    pub fn wrapping(&self) -> bool {
        self.word_wrapping
    }

    fn goto_prev_mark(&mut self) {
        let offset = self
            .filter_all_lines_iter()
            .into_iter()
            .enumerate()
            .take(self.line_offset)
            .filter(|(_, line)| line.1.marked)
            .map(|(i, _)| i)
            .last()
            .unwrap_or(self.line_offset);

        self.line_offset = offset;
    }

    fn goto_next_mark(&mut self) {
        let Some(offset) = self
            .filter_lines_iter()
            .into_iter()
            .skip(1)
            .enumerate()
            .find(|(_, line)| line.1.marked)
            .map(|(i, _)| i)
        else {
            return;
        };

        self.line_offset += offset + 1;
    }
}
