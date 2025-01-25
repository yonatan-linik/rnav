mod app_state;
mod command;
mod error;
mod log_file;
mod log_line;

use app_state::AppState;
use crossterm::event;
use error::Result;
use log_file::LogFile;
use ratatui::{
    layout::{Constraint, Layout},
    text::{Line, Text},
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
        Length(state.state_bar_text_number_of_lines() as u16),
    ]);
    let [title_bar, main_area, status_area, command_bar] = vertical.areas(frame.area());

    frame.render_widget(
        Line::styled(
            format!(
                "{} filters are currently active",
                state.total_filters_enabled()
            ),
            (ratatui::style::Color::Gray, ratatui::style::Color::DarkGray),
        ),
        status_area,
    );

    let mut command_bar_styled = state.command_bar_text();
    command_bar_styled.push_line(state.command_bar_completions());

    frame.render_widget(command_bar_styled, command_bar);
    frame.render_widget(Text::from_iter(state.lines_iter()), main_area);

    frame.render_widget(state.top_log_line_title_bar_text(), title_bar);
}
