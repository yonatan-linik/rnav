mod logline;

use color_eyre::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind,
};
use logline::LogLine;
use ratatui::{
    layout::{Constraint, Layout, Margin},
    text::Text,
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
    };

    loop {
        terminal.draw(|f| render(f, &mut state))?;
        match event::read()? {
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

    let vertical = Layout::vertical([Length(1), Min(0), Length(1)]);
    let [title_area, main_area, status_area] = vertical.areas(frame.area());
    let horizontal = Layout::horizontal([Fill(1); 1]);
    let [left_area] = horizontal.areas(main_area);

    frame.render_widget(
        Block::bordered().title(state.file_name.as_str()),
        title_area,
    );

    frame.render_widget(Block::bordered().title("Status Bar"), status_area);
    frame.render_widget(Block::bordered().title("Log contents"), left_area);
    frame.render_widget(
        Text::from_iter(state.lines.iter().map(|l| *l).skip(state.offset)),
        left_area.inner(Margin::new(1, 1)),
    );
    // frame.render_widget(Block::bordered().title("Right"), right_area);
    // frame.render_widget("hello world", Rect::new(0, 0, frame.area().width, 1));
    // frame.render_widget("another widget", Rect::new(0, 1, frame.area().width, 1));
}
