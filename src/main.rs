mod logline;

use color_eyre::{owo_colors::OwoColorize, Result};
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind,
};
use logline::LogLine;
use ratatui::{
    layout::{Constraint, Layout, Margin},
    text::{Text, ToText},
    widgets::Block,
    DefaultTerminal, Frame,
};

use clap::{command, Parser};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// File names to open
    file_names: Vec<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.file_names.is_empty() {
        eprintln!("Supply at least one file name");
        return Ok(());
    }
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = run(terminal, args);
    ratatui::restore();
    result
}

struct AppState<'a> {
    file_name: String,
    lines: Vec<LogLine<'a>>,
    offset: usize,
    filter_ins: Vec<String>,
    filter_outs: Vec<String>,
    command_mode: bool,
    current_command: String,
}

impl<'a> AppState<'a> {
    fn handle_command(&mut self) -> Option<()> {
        let mut curr_cmd = String::new();
        std::mem::swap(&mut curr_cmd, &mut self.current_command);
        let (command, arguments) = curr_cmd.trim().split_once(|c: char| c.is_whitespace())?;
        match command {
            "filter-in" => {
                self.filter_ins.push(arguments.to_string());
            }
            "filter-out" => {
                self.filter_outs.push(arguments.to_string());
            }
            _ => return None,
        }

        Some(())
    }
}

fn run(mut terminal: DefaultTerminal, args: Args) -> Result<()> {
    let first_file_name = args
        .file_names
        .first()
        .expect("First file name must exist")
        .as_str();

    let text = String::from_utf8(std::fs::read(first_file_name).expect("Can read file"))
        .expect("File is a valid utf-8 text file");
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

    let mut state = AppState {
        file_name: first_file_name.to_string(),
        lines,
        offset: 0,
        filter_ins: Vec::new(),
        filter_outs: Vec::new(),
        command_mode: false,
        current_command: String::new(),
    };

    loop {
        terminal.draw(|f| render(f, &mut state))?;
        let event = event::read()?;
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
            continue;
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
                break Ok(());
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
                        KeyCode::Char('R') => {
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
    }
}

fn render(frame: &mut Frame, state: &mut AppState) {
    use Constraint::{Fill, Length, Min};

    let vertical = Layout::vertical([Length(3), Min(1), Length(3)]);
    let [title_area, main_area, status_area] = vertical.areas(frame.area());
    let horizontal = Layout::horizontal([Fill(1); 1]);
    let [left_area] = horizontal.areas(main_area);

    frame.render_widget(
        Block::bordered().title(state.file_name.as_str()),
        title_area,
    );

    if state.command_mode {
        frame.render_widget(
            Text::styled(
                state.current_command.as_str(),
                ratatui::style::Style::default()
                    .fg(ratatui::style::Color::Rgb(255, 255, 255))
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            status_area.inner(Margin::new(1, 1)),
        );
    } else if !state.current_command.is_empty() {
        // Ignore bad commands for now
        let _ = state.handle_command();
    }
    // This has to come after handle command so the filters length is current
    frame.render_widget(
        Block::bordered().title("Status Bar").title_bottom(format!(
            "{} filters are currently active",
            state.filter_ins.len() + state.filter_outs.len()
        )),
        status_area,
    );
    frame.render_widget(Block::bordered().title("Log contents"), left_area);
    frame.render_widget(
        Text::from_iter(
            state
                .lines
                .iter()
                .map(|l| *l)
                .skip(state.offset)
                .filter(|l| {
                    state.filter_ins.is_empty()
                        || state.filter_ins.iter().any(|f| l.log.contains(f))
                })
                .filter(|l| {
                    state.filter_outs.is_empty()
                        || state.filter_outs.iter().all(|f| !l.log.contains(f))
                }),
        ),
        left_area.inner(Margin::new(1, 1)),
    );
}
