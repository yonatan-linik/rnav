mod app_state;
mod command;
mod error;
mod filter;
mod log_file;
mod log_line;

use app_state::{AppMode, AppState};
use crossterm::event;
use error::Result;
use log_file::LogFile;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Text},
    widgets::{Block, Borders, StatefulWidget},
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

fn run(mut terminal: DefaultTerminal, args: Args) -> Result<()> {
    let files: Vec<_> = args
        .file_names
        .iter()
        .map(|n| {
            LogFile::new_with_random_color(
                n.as_str().into(),
                String::from_utf8(std::fs::read(n).expect("Can read file"))
                    .expect("File is a valid utf-8 text file"),
            )
        })
        .collect();

    let mut state = AppState::new(&files);
    loop {
        terminal.draw(|f| render(f, &mut state))?;
        let event = event::read()?;
        match state.read_event(event) {
            app_state::AppAction::EndApp => break Ok(()),
            app_state::AppAction::NoAction => continue,
        }
    }
}

fn render(frame: &mut Frame, state: &mut AppState) {
    use Constraint::{Length, Min};

    let vertical = Layout::vertical([
        Length(1),
        Min(3),
        Length(1),
        // Min(2),
        Length(state.state_bar_text_number_of_lines() as u16),
    ]);
    let [title_bar, main_area, status_area, command_bar] = vertical.areas(frame.area());

    frame.render_widget(
        Line::styled(
            format!(
                "{} filters are currently active",
                state.filters.total_filters_enabled()
            ),
            (ratatui::style::Color::Gray, ratatui::style::Color::DarkGray),
        ),
        status_area,
    );

    match state.mode() {
        AppMode::Command => {
            frame.render_widget(state.command_bar_text(), command_bar);
        }
        AppMode::FiltersMenu => {
            let (info_lines, table, mut table_state) = state.filters.filters_menu_text();

            let table_area = Rect::new(
                command_bar.x,
                command_bar.y + state.filters.filters_menu_info_lines_size() as u16,
                command_bar.width,
                command_bar.height - 1,
            );
            frame.render_widget(info_lines, command_bar);
            table.render(table_area, frame.buffer_mut(), &mut table_state);
        }
        AppMode::Logs => (),
    }

    let b = Block::new().borders(Borders::RIGHT);
    frame.render_widget(&b, main_area);
    frame.render_widget(Text::from_iter(state.lines_iter()), b.inner(main_area));

    frame.render_widget(state.top_log_line_title_bar_text(), title_bar);
}
